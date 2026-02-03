use std::{error::Error, io::Cursor, path::Path};

use anchor_lang::{
    solana_program::{
        instruction::AccountMeta as AnchorAccountMeta, program_pack::Pack, system_program,
    },
    AccountDeserialize, AccountSerialize, InstructionData, Space, ToAccountMetas,
};
use anchor_spl::associated_token::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use anchor_spl::token;
use anchor_spl::token::spl_token::state::{
    Account as SplTokenAccount, AccountState, Mint as SplMint,
};
use kickstarter::{constants, state::{Kickstarter, PrivateFundState}};
use ephemeral_rollups_sdk::consts::{MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID};
use litesvm::LiteSVM;
use mpl_token_metadata::ID as MPL_TOKEN_METADATA_ID;
use solana_account::Account;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
pub use solana_pubkey::Pubkey;
pub use solana_signer::Signer;
use solana_transaction::Transaction;

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub const TEST_BASE_VAULT_BALANCE: u64 = 1_000_000_000_000_000_000;

pub fn program_artifact_path() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/kickstarter.so"
    )
}

pub fn mpl_artifact_path() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/metaplex_token_metadata_program.so"
    )
}

pub fn to_anchor_pubkey(pubkey: &Pubkey) -> anchor_lang::solana_program::pubkey::Pubkey {
    anchor_lang::solana_program::pubkey::Pubkey::new_from_array(pubkey.to_bytes())
}

pub fn to_solana_pubkey(pubkey: &anchor_lang::solana_program::pubkey::Pubkey) -> Pubkey {
    Pubkey::new_from_array(pubkey.to_bytes())
}

pub fn program_id() -> Pubkey {
    to_solana_pubkey(&kickstarter::id())
}

pub fn convert_metas(metas: Vec<AnchorAccountMeta>) -> Vec<AccountMeta> {
    metas
        .into_iter()
        .map(|meta| AccountMeta {
            pubkey: to_solana_pubkey(&meta.pubkey),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect()
}

pub fn pack_mint_account(mint: SplMint) -> Vec<u8> {
    let mut data = vec![0u8; <SplMint as Pack>::LEN];
    SplMint::pack(mint, &mut data).unwrap();
    data
}

pub fn pack_token_account(account: SplTokenAccount) -> Vec<u8> {
    let mut data = vec![0u8; <SplTokenAccount as Pack>::LEN];
    SplTokenAccount::pack(account, &mut data).unwrap();
    data
}

pub struct TestHarness {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
}

impl TestHarness {
    pub fn new() -> TestResult<Self> {
        let artifact = Path::new(program_artifact_path());
        if !artifact.exists() {
            return Err(format!("program artifact not found at {:?}", artifact).into());
        }

        let pid = program_id();
        let mut svm = LiteSVM::new().with_default_programs();
        svm.add_program_from_file(pid, artifact)
            .map_err(|err| format!("failed to add program: {err:?}"))?;

        let mpl_path = Path::new(mpl_artifact_path());
        if mpl_path.exists() {
            let mpl_id = Pubkey::new_from_array(MPL_TOKEN_METADATA_ID.to_bytes());
            svm.add_program_from_file(mpl_id, mpl_path)
                .map_err(|err| format!("failed to add mpl program: {err:?}"))?;
        } else {
            eprintln!("WARNING: Metaplex program not found at {:?}", mpl_path);
            eprintln!("         CPI calls to Token Metadata will fail.");
            eprintln!("         Run: solana program dump -u m metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s tests/metaplex_token_metadata_program.so");
        }

        Ok(Self {
            svm,
            program_id: pid,
        })
    }

    pub fn airdrop(&mut self, recipient: &Pubkey, amount: u64) -> TestResult {
        self.svm
            .airdrop(recipient, amount)
            .map_err(|err| format!("airdrop failed: {err:?}"))?;
        Ok(())
    }

    pub fn send(&mut self, ix: Instruction, signer: &Keypair) -> TestResult {
        let blockhash = self.svm.latest_blockhash();
        let message = Message::new(&[ix], Some(&signer.pubkey()));
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[signer], blockhash);
        self.svm
            .send_transaction(tx)
            .map_err(|err| format!("tx failed: {err:?}"))?;
        Ok(())
    }

    pub fn account(&self, key: &Pubkey) -> Option<Account> {
        self.svm.get_account(key)
    }

    pub fn kickstarter_state(&self, pda: &Pubkey) -> TestResult<Kickstarter> {
        let account = self.account(pda).ok_or("kickstarter account not found")?;
        let mut data_slice = account.data.as_slice();
        Ok(Kickstarter::try_deserialize(&mut data_slice)?)
    }

    pub fn private_state(&self, pda: &Pubkey) -> TestResult<PrivateFundState> {
        let account = self.account(pda).ok_or("private state account not found")?;
        let mut data_slice = account.data.as_slice();
        Ok(PrivateFundState::try_deserialize(&mut data_slice)?)
    }

    pub fn create_mock_mint(&mut self, mint: Pubkey, mint_authority: Pubkey) -> TestResult {
        let rent = self.svm.minimum_balance_for_rent_exemption(<SplMint as Pack>::LEN);
        let mint_data = pack_mint_account(SplMint {
            mint_authority: anchor_lang::solana_program::program_option::COption::Some(
                to_anchor_pubkey(&mint_authority),
            ),
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: anchor_lang::solana_program::program_option::COption::None,
        });
        self.svm
            .set_account(
                mint,
                Account {
                    lamports: rent,
                    data: mint_data,
                    owner: to_solana_pubkey(&token::ID),
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .map_err(|err| format!("failed to set mint account: {err:?}"))?;
        Ok(())
    }

    pub fn create_mock_token_account(
        &mut self,
        account_pubkey: Pubkey,
        mint: Pubkey,
        owner: Pubkey,
    ) -> TestResult {
        let rent = self.svm.minimum_balance_for_rent_exemption(<SplTokenAccount as Pack>::LEN);
        let account_data = pack_token_account(SplTokenAccount {
            mint: to_anchor_pubkey(&mint),
            owner: to_anchor_pubkey(&owner),
            amount: 0,
            delegate: anchor_lang::solana_program::program_option::COption::None,
            state: AccountState::Initialized,
            is_native: anchor_lang::solana_program::program_option::COption::None,
            delegated_amount: 0,
            close_authority: anchor_lang::solana_program::program_option::COption::None,
        });
        self.svm
            .set_account(
                account_pubkey,
                Account {
                    lamports: rent,
                    data: account_data,
                    owner: to_solana_pubkey(&token::ID),
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .map_err(|err| format!("failed to set token account: {err:?}"))?;
        Ok(())
    }

    pub fn set_token_account_balance(
        &mut self,
        account_pubkey: Pubkey,
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
    ) -> TestResult {
        let rent = self.svm.minimum_balance_for_rent_exemption(<SplTokenAccount as Pack>::LEN);
        let account_data = pack_token_account(SplTokenAccount {
            mint: to_anchor_pubkey(&mint),
            owner: to_anchor_pubkey(&owner),
            amount,
            delegate: anchor_lang::solana_program::program_option::COption::None,
            state: AccountState::Initialized,
            is_native: anchor_lang::solana_program::program_option::COption::None,
            delegated_amount: 0,
            close_authority: anchor_lang::solana_program::program_option::COption::None,
        });
        self.svm
            .set_account(
                account_pubkey,
                Account {
                    lamports: rent,
                    data: account_data,
                    owner: to_solana_pubkey(&token::ID),
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .map_err(|err| format!("failed to set token account balance: {err:?}"))?;
        Ok(())
    }

    pub fn update_kickstarter<F>(&mut self, pda: &Pubkey, updater: F) -> TestResult
    where
        F: FnOnce(&mut Kickstarter),
    {
        let mut ks_account = self.account(pda).ok_or("kickstarter account not found")?;
        let mut data_slice: &[u8] = &ks_account.data;
        let mut ks_state = Kickstarter::try_deserialize(&mut data_slice)?;

        updater(&mut ks_state);

        let required_size = 8 + Kickstarter::INIT_SPACE;
        if ks_account.data.len() < required_size {
            ks_account.data.resize(required_size, 0);
        }
        let mut cursor = Cursor::new(&mut ks_account.data[..]);
        ks_state.try_serialize(&mut cursor)?;
        self.svm
            .set_account(*pda, ks_account)
            .map_err(|e| format!("{:?}", e))?;
        Ok(())
    }
}

pub struct KickstarterAccounts {
    pub admin: Keypair,
    pub kickstarter_pda: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub private_state: Pubkey,
    pub treasury: Pubkey,
    pub treasury_token_account: Pubkey,
    pub liquidity_token_account: Pubkey,
    pub liquidity_base_token_account: Pubkey,
    pub metadata_pda: Pubkey,
}

impl KickstarterAccounts {
    pub fn generate(harness: &mut TestHarness) -> TestResult<Self> {
        let admin = Keypair::new();
        harness.airdrop(&admin.pubkey(), 10_000_000_000)?;

        let program_id = harness.program_id;
        let mpl_metadata_program_id = Pubkey::new_from_array(MPL_TOKEN_METADATA_ID.to_bytes());

        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();

        let (kickstarter_pda, _) = Pubkey::find_program_address(
            &[b"kickstarter", admin.pubkey().as_ref(), base_mint.as_ref()],
            &program_id,
        );

        harness.create_mock_mint(base_mint, kickstarter_pda)?;
        harness.create_mock_mint(quote_mint, admin.pubkey())?;

        let (base_vault, _) = Pubkey::find_program_address(
            &[constants::SEED_BASE_VAULT.as_bytes(), kickstarter_pda.as_ref()],
            &program_id,
        );
        let (quote_vault, _) = Pubkey::find_program_address(
            &[constants::SEED_QUOTE_VAULT.as_bytes(), kickstarter_pda.as_ref()],
            &program_id,
        );
        let (private_state, _) = Pubkey::find_program_address(
            &[constants::SEED_PRIVATE_STATE.as_bytes(), kickstarter_pda.as_ref()],
            &program_id,
        );

        // In CLI, treasury = admin, so we use admin.pubkey() as treasury
        let treasury = admin.pubkey();
        let (metadata_pda, _) = Pubkey::find_program_address(
            &[b"metadata", MPL_TOKEN_METADATA_ID.as_ref(), base_mint.as_ref()],
            &mpl_metadata_program_id,
        );

        let treasury_token_account = Pubkey::new_unique();
        harness.create_mock_token_account(treasury_token_account, quote_mint, treasury)?;

        let liquidity_token_account = Pubkey::new_unique();
        harness.create_mock_token_account(liquidity_token_account, quote_mint, admin.pubkey())?;

        let liquidity_base_token_account = Pubkey::new_unique();
        harness.create_mock_token_account(liquidity_base_token_account, base_mint, admin.pubkey())?;

        Ok(Self {
            admin,
            kickstarter_pda,
            base_mint,
            quote_mint,
            base_vault,
            quote_vault,
            private_state,
            treasury,
            treasury_token_account,
            liquidity_token_account,
            liquidity_base_token_account,
            metadata_pda,
        })
    }

    pub fn mpl_program_id(&self) -> Pubkey {
        Pubkey::new_from_array(MPL_TOKEN_METADATA_ID.to_bytes())
    }

    pub fn rent_id(&self) -> Pubkey {
        Pubkey::new_from_array(anchor_lang::solana_program::sysvar::rent::ID.to_bytes())
    }

    pub fn initialize_ix(&self, config: InitConfig) -> Instruction {
        let accounts = kickstarter::accounts::InitializeKickstarter {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
            base_mint: to_anchor_pubkey(&self.base_mint),
            quote_mint: to_anchor_pubkey(&self.quote_mint),
            base_vault: to_anchor_pubkey(&self.base_vault),
            quote_vault: to_anchor_pubkey(&self.quote_vault),
            private_state: to_anchor_pubkey(&self.private_state),
            treasury: to_anchor_pubkey(&self.treasury),
            token_program: anchor_spl::token::ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
            metadata: to_anchor_pubkey(&self.metadata_pda),
            token_metadata_program: to_anchor_pubkey(&self.mpl_program_id()),
            rent: to_anchor_pubkey(&self.rent_id()),
        };

        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::InitializeKickstarter {
                minimum_raise_amount: config.minimum_raise_amount,
                total_base_tokens_for_investors: config.total_base_tokens_for_investors,
                performance_pool_base_tokens: config.performance_pool_base_tokens,
                seconds_for_launch: config.seconds_for_launch,
                monthly_team_spending_usdc: config.monthly_team_spending_usdc,
                package_unlock_delay_seconds: config.package_unlock_delay_seconds,
                token_name: config.token_name,
                token_symbol: config.token_symbol,
                token_description: config.token_description,
                token_image_url: config.token_image_url,
            }
            .data(),
        }
    }

    pub fn start_ix(&self) -> Instruction {
        let accounts = kickstarter::accounts::StartKickstarter {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::StartKickstarter {}.data(),
        }
    }

    pub fn complete_ix(&self, final_raise_amount: u64) -> Instruction {
        let accounts = kickstarter::accounts::CompleteKickstarter {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
            quote_vault: to_anchor_pubkey(&self.quote_vault),
            treasury_token_account: to_anchor_pubkey(&self.treasury_token_account),
            liquidity_token_account: to_anchor_pubkey(&self.liquidity_token_account),
            base_vault: to_anchor_pubkey(&self.base_vault),
            liquidity_base_token_account: to_anchor_pubkey(&self.liquidity_base_token_account),
            base_mint: to_anchor_pubkey(&self.base_mint),
            token_program: anchor_spl::token::ID,
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::CompleteKickstarter { final_raise_amount }.data(),
        }
    }

    pub fn configure_performance_package_ix(&self, index: u8, multiplier: u8, allocation: u64) -> Instruction {
        let accounts = kickstarter::accounts::ConfigurePerformancePackage {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::ConfigurePerformancePackage {
                index,
                multiplier,
                allocation,
            }
            .data(),
        }
    }

    pub fn unlock_performance_package_ix(&self, index: u8, current_price: u64) -> Instruction {
        let accounts = kickstarter::accounts::UnlockPerformancePackage {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::UnlockPerformancePackage { index, current_price }.data(),
        }
    }

    pub fn claim_performance_package_ix(&self, index: u8, recipient_base_account: Pubkey) -> Instruction {
        let accounts = kickstarter::accounts::ClaimPerformancePackage {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
            base_vault: to_anchor_pubkey(&self.base_vault),
            recipient_base_account: to_anchor_pubkey(&recipient_base_account),
            token_program: anchor_spl::token::ID,
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::ClaimPerformancePackage { index }.data(),
        }
    }

    pub fn close_ix(&self) -> Instruction {
        let accounts = kickstarter::accounts::CloseKickstarter {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::CloseKickstarter {}.data(),
        }
    }

    pub fn set_minimum_raise_ix(&self, new_minimum: u64) -> Instruction {
        let accounts = kickstarter::accounts::SetMinimumRaise {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::SetMinimumRaise { new_minimum_raise: new_minimum }.data(),
        }
    }

    pub fn stake_from_treasury_ix(&self, staking_account: Pubkey, amount: u64) -> Instruction {
        let accounts = kickstarter::accounts::StakeFromTreasury {
            admin: to_anchor_pubkey(&self.admin.pubkey()),
            kickstarter: to_anchor_pubkey(&self.kickstarter_pda),
            treasury_token_account: to_anchor_pubkey(&self.treasury_token_account),
            staking_account: to_anchor_pubkey(&staking_account),
            token_program: anchor_spl::token::ID,
        };
        Instruction {
            program_id: program_id(),
            accounts: convert_metas(accounts.to_account_metas(Some(true))),
            data: kickstarter::instruction::StakeFromTreasury { amount }.data(),
        }
    }
}

pub fn derive_funder_position(kickstarter_pda: &Pubkey, user: &Pubkey) -> Pubkey {
    let (pda, _) = Pubkey::find_program_address(
        &[b"funder_position", kickstarter_pda.as_ref(), user.as_ref()],
        &program_id(),
    );
    pda
}

pub fn fund_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    user_quote_account: &Pubkey,
    quote_vault: &Pubkey,
    amount: u64,
) -> Instruction {
    let funder_position = derive_funder_position(kickstarter_pda, user);
    let accounts = kickstarter::accounts::Fund {
        funder: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        funder_position: to_anchor_pubkey(&funder_position),
        funder_quote_account: to_anchor_pubkey(user_quote_account),
        quote_vault: to_anchor_pubkey(quote_vault),
        token_program: anchor_spl::token::ID,
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::Fund { amount }.data(),
    }
}

pub fn fund_private_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    private_state: &Pubkey,
    amount: u64,
    salt: [u8; 32],
) -> Instruction {
    let accounts = kickstarter::accounts::FundPrivate {
        funder: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        private_state: to_anchor_pubkey(private_state),
        magic_context: MAGIC_CONTEXT_ID,
        magic_program: MAGIC_PROGRAM_ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::FundPrivate { amount, salt }.data(),
    }
}

pub fn finalize_private_round_ix(
    admin: &Pubkey,
    kickstarter_pda: &Pubkey,
    private_state: &Pubkey,
    final_commitments_root: [u8; 32],
    attested_total_amount: u64,
    attestation_signature: [u8; 64],
) -> Instruction {
    let accounts = kickstarter::accounts::FinalizePrivateRound {
        admin: to_anchor_pubkey(admin),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        private_state: to_anchor_pubkey(private_state),
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::FinalizePrivateRound {
            final_commitments_root,
            attested_total_amount,
            attestation_signature,
        }.data(),
    }
}

pub fn private_claim_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    private_state: &Pubkey,
    base_vault: &Pubkey,
    user_base_account: &Pubkey,
    amount: u64,
    salt: [u8; 32],
) -> Instruction {
    let accounts = kickstarter::accounts::PrivateClaim {
        user: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        private_state: to_anchor_pubkey(private_state),
        base_vault: to_anchor_pubkey(base_vault),
        user_base_account: to_anchor_pubkey(user_base_account),
        token_program: anchor_spl::token::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::PrivateClaim { amount, salt }.data(),
    }
}

pub fn private_compressed_claim_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    private_state: &Pubkey,
    base_vault: &Pubkey,
    compressed_token_account: &Pubkey,
    amount: u64,
    salt: [u8; 32],
) -> Instruction {
    let accounts = kickstarter::accounts::PrivateClaimCompressed {
        user: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        private_state: to_anchor_pubkey(private_state),
        base_vault: to_anchor_pubkey(base_vault),
        compressed_token_account: to_anchor_pubkey(compressed_token_account),
        token_program: anchor_spl::token::ID,
        compression_program: to_anchor_pubkey(&Pubkey::new_unique()), // Placeholder
        system_program: anchor_lang::solana_program::system_program::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::PrivateClaimCompressed { amount, salt }.data(),
    }
}

pub fn private_refund_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    private_state: &Pubkey,
    quote_vault: &Pubkey,
    user_quote_account: &Pubkey,
    amount: u64,
    salt: [u8; 32],
) -> Instruction {
    let accounts = kickstarter::accounts::PrivateRefund {
        user: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        private_state: to_anchor_pubkey(private_state),
        quote_vault: to_anchor_pubkey(quote_vault),
        user_quote_account: to_anchor_pubkey(user_quote_account),
        token_program: anchor_spl::token::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::PrivateRefund { amount, salt }.data(),
    }
}

pub fn start_private_round_ix(admin: &Pubkey, kickstarter_pda: &Pubkey) -> Instruction {
    let accounts = kickstarter::accounts::StartPrivateRound {
        admin: to_anchor_pubkey(admin),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::StartPrivateRound {}.data(),
    }
}

pub fn end_private_round_ix(admin: &Pubkey, kickstarter_pda: &Pubkey) -> Instruction {
    let accounts = kickstarter::accounts::EndPrivateRound {
        admin: to_anchor_pubkey(admin),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::EndPrivateRound {}.data(),
    }
}

pub fn claim_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    base_vault: &Pubkey,
    user_base_account: &Pubkey,
) -> Instruction {
    let funder_position = derive_funder_position(kickstarter_pda, user);
    let accounts = kickstarter::accounts::Claim {
        user: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        funder_position: to_anchor_pubkey(&funder_position),
        base_vault: to_anchor_pubkey(base_vault),
        user_base_account: to_anchor_pubkey(user_base_account),
        token_program: anchor_spl::token::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::Claim {}.data(),
    }
}

pub fn refund_ix(
    user: &Pubkey,
    kickstarter_pda: &Pubkey,
    quote_vault: &Pubkey,
    user_quote_account: &Pubkey,
) -> Instruction {
    let funder_position = derive_funder_position(kickstarter_pda, user);
    let accounts = kickstarter::accounts::Refund {
        user: to_anchor_pubkey(user),
        kickstarter: to_anchor_pubkey(kickstarter_pda),
        funder_position: to_anchor_pubkey(&funder_position),
        quote_vault: to_anchor_pubkey(quote_vault),
        user_quote_account: to_anchor_pubkey(user_quote_account),
        token_program: anchor_spl::token::ID,
    };
    Instruction {
        program_id: program_id(),
        accounts: convert_metas(accounts.to_account_metas(Some(true))),
        data: kickstarter::instruction::Refund {}.data(),
    }
}

pub struct InitConfig {
    pub minimum_raise_amount: u64,
    pub total_base_tokens_for_investors: u64,
    pub performance_pool_base_tokens: u64,
    pub seconds_for_launch: u32,
    pub monthly_team_spending_usdc: u64,
    pub package_unlock_delay_seconds: i64,
    pub token_name: String,
    pub token_symbol: String,
    pub token_description: String,
    pub token_image_url: String,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            minimum_raise_amount: 1_000_000,
            total_base_tokens_for_investors: 10_000_000_000_000,
            performance_pool_base_tokens: 0,
            seconds_for_launch: 3600,
            monthly_team_spending_usdc: 0,
            package_unlock_delay_seconds: 0,
            token_name: "Test Token".to_string(),
            token_symbol: "TEST".to_string(),
            token_description: "Description".to_string(),
            token_image_url: "https://example.com/image.png".to_string(),
        }
    }
}

