use anchor_lang::prelude::*;
use sha2::{Digest, Sha256};

pub const MAX_COMMITMENTS: usize = 256;
pub const MAX_LEAVES_SIZE: usize = MAX_COMMITMENTS * 32;

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct MerkleTree {
    #[max_len(MAX_COMMITMENTS)]
    pub leaves: Vec<[u8; 32]>,
    pub root: [u8; 32],
    pub count: u32,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self {
            leaves: Vec::new(),
            root: [0u8; 32],
            count: 0,
        }
    }
}

impl MerkleTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_commitment(&mut self, commitment: [u8; 32]) -> Result<()> {
        require!(self.leaves.len() < MAX_COMMITMENTS, ErrorCode::InvalidCommitmentsRoot);

        self.leaves.push(commitment);
        self.count += 1;
        self.update_root();

        Ok(())
    }

    pub fn update_root(&mut self) {
        if self.leaves.is_empty() {
            self.root = [0u8; 32];
            return;
        }

        let mut current_level = self.leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    left
                };

                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                let hash = hasher.finalize();
                next_level.push(hash.into());
            }

            current_level = next_level;
        }

        self.root = current_level[0];
    }

    pub fn verify_commitment(&self, commitment: [u8; 32], index: usize) -> bool {
        if index >= self.leaves.len() || self.leaves[index] != commitment {
            return false;
        }

        let mut current_hash = commitment;
        let mut current_index = index;

        let mut level = self.leaves.clone();

        while level.len() > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            let sibling_hash = if sibling_index < level.len() {
                level[sibling_index]
            } else {
                current_hash
            };

            let mut hasher = Sha256::new();
            if current_index % 2 == 0 {
                hasher.update(current_hash);
                hasher.update(sibling_hash);
            } else {
                hasher.update(sibling_hash);
                hasher.update(current_hash);
            }

            current_hash = hasher.finalize().into();
            current_index /= 2;

            let mut next_level = Vec::new();
            for i in (0..level.len()).step_by(2) {
                let left = level[i];
                let right = if i + 1 < level.len() {
                    level[i + 1]
                } else {
                    left
                };

                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                next_level.push(hasher.finalize().into());
            }
            level = next_level;
        }

        current_hash == self.root
    }

    pub fn get_root(&self) -> [u8; 32] {
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_basic() {
        let mut tree = MerkleTree::new();

        let commitment1 = [1u8; 32];
        let commitment2 = [2u8; 32];

        tree.add_commitment(commitment1).unwrap();
        tree.add_commitment(commitment2).unwrap();

        assert_eq!(tree.count, 2);
        assert!(tree.verify_commitment(commitment1, 0));
        assert!(tree.verify_commitment(commitment2, 1));
    }
}