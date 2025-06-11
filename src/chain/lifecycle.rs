// Module for chain lifecycle management (currently minimal)
use std::collections::HashMap;

use crate::core::{
    errors::{HashChainError, HashChainResult},
    types::*,
    utils::get_current_timestamp,
};

/// Chain lifecycle management
#[derive(Clone)]
pub struct ChainLifecycle {
    /// Chain identifier
    pub chain_id: ChainId,
    /// Current state
    pub state: ChainState,
    /// Created timestamp
    pub created_at: f64,
    /// Last update timestamp
    pub updated_at: f64,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Chain states
#[derive(Clone, Debug)]
pub enum ChainState {
    /// Chain is being initialized
    Initializing,
    /// Chain is active and processing blocks
    Active,
    /// Chain is paused (not processing new blocks)
    Paused,
    /// Chain is being archived
    Archiving,
    /// Chain has been archived
    Archived,
    /// Chain has been removed
    Removed,
}

impl ChainLifecycle {
    pub fn new(chain_id: ChainId) -> Self {
        let now = get_current_timestamp();
        Self {
            chain_id,
            state: ChainState::Initializing,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn activate(&mut self) -> HashChainResult<()> {
        match self.state {
            ChainState::Initializing | ChainState::Paused => {
                self.state = ChainState::Active;
                self.updated_at = get_current_timestamp();
                Ok(())
            }
            _ => Err(HashChainError::ChainLifecycle {
                reason: format!("Cannot activate chain in state: {:?}", self.state),
            }),
        }
    }

    pub fn pause(&mut self) -> HashChainResult<()> {
        match self.state {
            ChainState::Active => {
                self.state = ChainState::Paused;
                self.updated_at = get_current_timestamp();
                Ok(())
            }
            _ => Err(HashChainError::ChainLifecycle {
                reason: format!("Cannot pause chain in state: {:?}", self.state),
            }),
        }
    }

    pub fn archive(&mut self) -> HashChainResult<()> {
        match self.state {
            ChainState::Active | ChainState::Paused => {
                self.state = ChainState::Archiving;
                self.updated_at = get_current_timestamp();
                Ok(())
            }
            _ => Err(HashChainError::ChainLifecycle {
                reason: format!("Cannot archive chain in state: {:?}", self.state),
            }),
        }
    }

    pub fn remove(&mut self) -> HashChainResult<()> {
        self.state = ChainState::Removed;
        self.updated_at = get_current_timestamp();
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, ChainState::Active)
    }

    pub fn get_state(&self) -> &ChainState {
        &self.state
    }
}
