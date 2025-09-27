use crate::constants::ZERO_HASH;
use crate::store::{Store, Vote};
use std::collections::HashMap;
use types::{EthSpec, Hash256};

//TODO(lean): bad GPT stuff deal with it
pub fn get_fork_choice_head<E: EthSpec>(
    store: &Store<E>,
    justified_root: Hash256,
) -> Result<Hash256, String> {
    get_head_from_votes(&store.latest_messages, &store.blocks, justified_root)
}

pub fn get_latest_justified<E: EthSpec>(store: &Store<E>) -> Result<types::Checkpoint, String> {
    Ok(store.best_justified_checkpoint)
}

fn get_head_from_votes<E: EthSpec>(
    votes: &HashMap<u64, Vote>,
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    start_root: Hash256,
) -> Result<Hash256, String> {
    let vote_weights = calculate_vote_weights(votes, blocks, start_root)?;

    let children_map = build_children_map(blocks);

    let mut current = start_root;

    loop {
        let children = children_map.get(&current).cloned().unwrap_or_default();

        if children.is_empty() {
            return Ok(current);
        }

        let mut best_child = children[0];
        let mut best_weight = vote_weights.get(&children[0]).copied().unwrap_or(0);

        for &child in &children[1..] {
            let weight = vote_weights.get(&child).copied().unwrap_or(0);

            if weight > best_weight || (weight == best_weight && child > best_child) {
                best_child = child;
                best_weight = weight;
            }
        }

        current = best_child;
    }
}

fn calculate_vote_weights<E: EthSpec>(
    votes: &HashMap<u64, Vote>,
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    start_root: Hash256,
) -> Result<HashMap<Hash256, u64>, String> {
    let mut weights = HashMap::new();

    for &block_root in blocks.keys() {
        weights.insert(block_root, 0u64);
    }

    for vote in votes.values() {
        if let Some(weight) = weights.get_mut(&vote.current_root) {
            *weight = weight.saturating_add(1);
        }
    }

    propagate_weights_upward(&mut weights, blocks, start_root)?;

    Ok(weights)
}

fn propagate_weights_upward<E: EthSpec>(
    weights: &mut HashMap<Hash256, u64>,
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    start_root: Hash256,
) -> Result<(), String> {
    let mut ordered_blocks = Vec::new();
    collect_blocks_postorder(blocks, start_root, &mut ordered_blocks)?;

    for block_root in ordered_blocks {
        if let Some(block) = blocks.get(&block_root) {
            let parent_root = block.parent_root();
            if parent_root != ZERO_HASH {
                let block_weight = weights.get(&block_root).copied().unwrap_or(0);
                if let Some(parent_weight) = weights.get_mut(&parent_root) {
                    *parent_weight = parent_weight.saturating_add(block_weight);
                }
            }
        }
    }

    Ok(())
}

fn collect_blocks_postorder<E: EthSpec>(
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    root: Hash256,
    ordered: &mut Vec<Hash256>,
) -> Result<(), String> {
    let children_map = build_children_map(blocks);

    if let Some(children) = children_map.get(&root) {
        for &child in children {
            collect_blocks_postorder(blocks, child, ordered)?;
        }
    }

    ordered.push(root);
    Ok(())
}

fn build_children_map<E: EthSpec>(
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
) -> HashMap<Hash256, Vec<Hash256>> {
    let mut children_map: HashMap<Hash256, Vec<Hash256>> = HashMap::new();

    for (&block_root, block) in blocks {
        let parent_root = block.parent_root();
        if parent_root != ZERO_HASH {
            children_map
                .entry(parent_root)
                .or_default()
                .push(block_root);
        }
    }

    for children in children_map.values_mut() {
        children.sort();
    }

    children_map
}

pub fn is_ancestor<E: EthSpec>(
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    ancestor: Hash256,
    descendant: Hash256,
) -> bool {
    if ancestor == descendant {
        return true;
    }

    let mut current = descendant;
    while let Some(block) = blocks.get(&current) {
        let parent = block.parent_root();
        if parent == ancestor {
            return true;
        }
        if parent == ZERO_HASH {
            return false;
        }
        current = parent;
    }
    false
}

/// Get all ancestors of a block up to a given root
pub fn get_ancestors<E: EthSpec>(
    blocks: &HashMap<Hash256, types::BeaconBlock<E>>,
    block_root: Hash256,
    stop_root: Hash256,
) -> Vec<Hash256> {
    let mut ancestors = Vec::new();
    let mut current = block_root;

    while current != stop_root {
        if let Some(block) = blocks.get(&current) {
            let parent = block.parent_root();
            if parent == ZERO_HASH {
                break;
            }
            ancestors.push(parent);
            current = parent;
        } else {
            break;
        }
    }

    ancestors
}
