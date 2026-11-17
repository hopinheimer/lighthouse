use super::*;

pub fn get_execution_status_test_definition_01() -> ForkChoiceTestDefinition {
    let balances = vec![1; 2];
    let ops = vec![
        // Ensure that the head starts at the finalized block.
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(0),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 2.
        //
        //          0
        //         /
        //        2
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(2),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is 2
        //
        //          0
        //         /
        // head-> 2
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 1 that comes off the genesis block (this is a fork compared
        // to the previous block).
        //
        //          0
        //         / \
        //        2   1
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(1),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a vote to block 1
        //
        //          0
        //         / \
        //        2   1 <- +vote
        Operation::ProcessAttestation {
            validator_index: 0,
            block_root: get_root(1),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is now 1, because 1 has a vote.
        //
        //          0
        //         / \
        //        2   1 <- head
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(1),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Add a vote to block 2
        //
        //           0
        //          / \
        // +vote-> 2   1
        Operation::ProcessAttestation {
            validator_index: 1,
            block_root: get_root(2),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is 2 since 1 and 2 both have a vote
        //
        //          0
        //         / \
        // head-> 2   1
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        // Add block 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3
        Operation::ProcessBlock {
            slot: Slot::new(2),
            root: get_root(3),
            parent_root: get_root(1),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        //            |
        //            3
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
        // Move validator #0 vote from 1 to 3
        //
        //          0
        //         / \
        //        2   1 <- -vote
        //            |
        //            3 <- +vote
        Operation::ProcessAttestation {
            validator_index: 0,
            block_root: get_root(3),
            attestation_slot: Slot::new(3),
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        //            |
        //            3
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 1,
        },
        // Invalidate the payload of 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3 <- INVALID
        Operation::InvalidatePayload {
            head_hash: get_hash(3),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        // Ensure that the head is still 2.
        //
        //          0
        //         / \
        // head-> 2   1
        //            |
        //            3 <- INVALID
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Invalidation of 3 should have removed upstream weight.
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 1,
        },
        // Invalidation of 3 should have removed upstream weight.
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 0,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        // Invalidation should have removed weight.
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
        // Move a vote from 2 to 1. This is slashable, but that's not relevant here.
        //
        //           0
        //          / \
        // -vote-> 2   1 <- +vote
        //             |
        //             3 <- INVALID
        Operation::ProcessAttestation {
            validator_index: 1,
            block_root: get_root(1),
            attestation_slot: Slot::new(3),
        },
        // Ensure that the head has switched back to 1
        //
        //          0
        //         / \
        //        2   1 <-head
        //            |
        //            3 <- INVALID
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances,
            expected_head: get_root(1),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Invalidation should have removed weight.
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
    ];

    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        finalized_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        operations: ops,
        execution_payload_parent_hash: None,
        execution_payload_block_hash: None,
        spec: None,
    }
}

pub fn get_execution_status_test_definition_02() -> ForkChoiceTestDefinition {
    let balances = vec![1; 2];
    let ops = vec![
        // Ensure that the head starts at the finalized block.
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(0),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 2.
        //
        //          0
        //         /
        //        2
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(2),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is 2
        //
        //          0
        //         /
        // head-> 2
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 1 that comes off the genesis block (this is a fork compared
        // to the previous block).
        //
        //          0
        //         / \
        //        2   1
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(1),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a vote to block 1
        //
        //          0
        //         / \
        //        2   1 <- +vote
        Operation::ProcessAttestation {
            validator_index: 0,
            block_root: get_root(1),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is now 1, because 1 has a vote.
        //
        //          0
        //         / \
        //        2   1 <- head
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(1),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Add a vote to block 2
        //
        //           0
        //          / \
        // +vote-> 2   1
        Operation::ProcessAttestation {
            validator_index: 1,
            block_root: get_root(2),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is 2 since 1 and 2 both have a vote
        //
        //          0
        //         / \
        // head-> 2   1
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        // Add block 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3
        Operation::ProcessBlock {
            slot: Slot::new(2),
            root: get_root(3),
            parent_root: get_root(1),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        //            |
        //            3
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 1,
        },
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
        // Move validator #0 vote from 1 to 3
        //
        //          0
        //         / \
        //        2   1 <- -vote
        //            |
        //            3 <- +vote
        Operation::ProcessAttestation {
            validator_index: 0,
            block_root: get_root(3),
            attestation_slot: Slot::new(3),
        },
        // Move validator #1 vote from 2 to 3
        //
        //          0
        //         / \
        // -vote->2   1
        //            |
        //            3 <- +vote
        Operation::ProcessAttestation {
            validator_index: 1,
            block_root: get_root(3),
            attestation_slot: Slot::new(3),
        },
        // Ensure that the head is now 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3 <-head
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(3),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 2,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 2,
        },
        // Invalidate the payload of 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3 <- INVALID
        Operation::InvalidatePayload {
            head_hash: get_hash(3),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        // Ensure that the head is now 2.
        //
        //          0
        //         / \
        // head-> 2   1
        //            |
        //            3 <- INVALID
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances,
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Invalidation of 3 should have removed upstream weight.
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 0,
        },
        // Invalidation of 3 should have removed upstream weight.
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 0,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Invalidation should have removed weight.
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
    ];

    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        finalized_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        operations: ops,
        execution_payload_parent_hash: None,
        execution_payload_block_hash: None,
        spec: None,
    }
}

pub fn get_execution_status_test_definition_03() -> ForkChoiceTestDefinition {
    let balances = vec![1_000; 2_000];
    let ops = vec![
        // Ensure that the head starts at the finalized block.
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(0),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 2.
        //
        //          0
        //         /
        //        2
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(2),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is 2
        //
        //          0
        //         /
        // head-> 2
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a block with a hash of 1 that comes off the genesis block (this is a fork compared
        // to the previous block).
        //
        //          0
        //         / \
        //        2   1
        Operation::ProcessBlock {
            slot: Slot::new(1),
            root: get_root(1),
            parent_root: get_root(0),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is still 2
        //
        //          0
        //         / \
        // head-> 2   1
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(2),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        // Add a vote to block 1
        //
        //          0
        //         / \
        //        2   1 <- +vote
        Operation::ProcessAttestation {
            validator_index: 0,
            block_root: get_root(1),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is now 1, because 1 has a vote.
        //
        //          0
        //         / \
        //        2   1 <- head
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(1),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 1_000,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 1_000,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Add another vote to 1
        //
        //           0
        //          / \
        //         2   1 <- +vote
        Operation::ProcessAttestation {
            validator_index: 1,
            block_root: get_root(1),
            attestation_slot: Slot::new(2),
        },
        // Ensure that the head is 1.
        //
        //          0
        //         / \
        //        2   1 <- head
        Operation::FindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(1),
            current_slot: Slot::new(0),
            expected_payload_status: None,
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // Add block 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3
        Operation::ProcessBlock {
            slot: Slot::new(2),
            root: get_root(3),
            parent_root: get_root(1),
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            execution_payload_parent_hash: None,
            execution_payload_block_hash: None,
        },
        // Ensure that the head is now 3, applying a proposer boost to 3 as well.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3 <- head
        Operation::ProposerBoostFindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances.clone(),
            expected_head: get_root(3),
            proposer_boost_root: get_root(3),
        },
        // Stored weights are pure attestation scores (proposer boost is applied
        // on-the-fly in the walk's `get_weight`, not baked into `node.weight()`).
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
        // Invalidate the payload of 3.
        //
        //          0
        //         / \
        //        2   1
        //            |
        //            3 <- INVALID
        Operation::InvalidatePayload {
            head_hash: get_hash(3),
            latest_valid_ancestor: Some(get_hash(1)),
        },
        // Ensure that the head is now 1, maintaining the proposer boost on the invalid block.
        //
        //          0
        //         / \
        //        2   1 <- head
        //            |
        //            3 <- INVALID
        Operation::ProposerBoostFindHead {
            justified_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            finalized_checkpoint: Checkpoint {
                epoch: Epoch::new(1),
                root: get_root(0),
            },
            justified_state_balances: balances,
            expected_head: get_root(1),
            proposer_boost_root: get_root(3),
        },
        Operation::AssertWeight {
            block_root: get_root(0),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(1),
            weight: 2_000,
        },
        Operation::AssertWeight {
            block_root: get_root(2),
            weight: 0,
        },
        // The proposer boost should be reverted due to the invalid payload.
        Operation::AssertWeight {
            block_root: get_root(3),
            weight: 0,
        },
    ];

    ForkChoiceTestDefinition {
        finalized_block_slot: Slot::new(0),
        justified_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        finalized_checkpoint: Checkpoint {
            epoch: Epoch::new(1),
            root: get_root(0),
        },
        operations: ops,
        execution_payload_parent_hash: None,
        execution_payload_block_hash: None,
        spec: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_01() {
        let test = get_execution_status_test_definition_01();
        test.run();
    }

    #[test]
    fn test_02() {
        let test = get_execution_status_test_definition_02();
        test.run();
    }

    #[test]
    fn test_03() {
        let test = get_execution_status_test_definition_03();
        test.run();
    }

    /// The cases below run on pre-Gloas blocks, which carry their payload, and on Gloas blocks,
    /// whose payload arrives in an envelope.
    fn definition(gloas: bool, operations: Vec<Operation>) -> ForkChoiceTestDefinition {
        let spec = gloas.then(|| {
            let mut spec = MainnetEthSpec::default_spec();
            spec.proposer_score_boost = 50;
            spec.gloas_fork_epoch = Some(Epoch::new(0));
            spec
        });
        ForkChoiceTestDefinition {
            finalized_block_slot: Slot::new(0),
            justified_checkpoint: get_checkpoint(0),
            finalized_checkpoint: get_checkpoint(0),
            operations,
            // Gloas children of genesis that name hash 42 extend its `EMPTY` node.
            execution_payload_parent_hash: gloas.then(|| get_hash(42)),
            execution_payload_block_hash: gloas.then(|| get_hash(0)),
            spec,
        }
    }

    /// `genesis <- 1 <- 2`, each block executing its parent's payload. A Gloas block's envelope
    /// arrives unjudged, so on both forks every payload starts `Optimistic`.
    fn chain_of_two(gloas: bool) -> Vec<Operation> {
        let mut operations = vec![];
        for (root, parent_hash) in [(1, 42), (2, 1)] {
            operations.push(Operation::ProcessBlock {
                slot: Slot::new(root),
                root: get_root(root),
                parent_root: get_root(root - 1),
                justified_checkpoint: get_checkpoint(0),
                finalized_checkpoint: get_checkpoint(0),
                execution_payload_parent_hash: gloas.then(|| get_hash(parent_hash)),
                execution_payload_block_hash: gloas.then(|| get_hash(root)),
            });
            if gloas {
                operations.push(Operation::ProcessUnjudgedExecutionPayloadEnvelope {
                    block_root: get_root(root),
                });
            }
        }
        operations
    }

    fn status(root: u64, expected: fn(ExecutionBlockHash) -> ExecutionStatus) -> Operation {
        Operation::AssertExecutionStatus {
            block_root: get_root(root),
            expected: expected(get_hash(root)),
        }
    }

    /// A payload rejected on `newPayload` never reaches fork choice, so the walk starts at the
    /// payload it builds on. That payload is condemned only when the latest valid hash lies
    /// beneath it, and an unknown parent payload is ignored.
    fn rejected_on_import(gloas: bool) -> ForkChoiceTestDefinition {
        let mut operations = chain_of_two(gloas);
        operations.extend([
            Operation::InvalidatePayloadOnImport {
                parent_hash: get_hash(2),
                latest_valid_ancestor: get_hash(2),
            },
            status(2, ExecutionStatus::Optimistic),
            Operation::InvalidatePayloadOnImport {
                parent_hash: get_hash(99),
                latest_valid_ancestor: get_hash(1),
            },
            status(2, ExecutionStatus::Optimistic),
            Operation::InvalidatePayloadOnImport {
                parent_hash: get_hash(2),
                latest_valid_ancestor: get_hash(1),
            },
            status(1, ExecutionStatus::Optimistic),
            status(2, ExecutionStatus::Invalid),
        ]);
        definition(gloas, operations)
    }

    /// VALID on a payload whose ancestry was condemned is a contradiction from the execution
    /// layer: it is refused and every status stays as it was.
    fn valid_after_invalid_ancestor(gloas: bool) -> ForkChoiceTestDefinition {
        let mut operations = chain_of_two(gloas);
        operations.extend([
            Operation::InvalidatePayload {
                head_hash: get_hash(1),
                latest_valid_ancestor: None,
            },
            status(2, ExecutionStatus::Invalid),
            Operation::ValidatePayload {
                block_hash: get_hash(2),
                expect_error: true,
            },
            status(1, ExecutionStatus::Invalid),
            status(2, ExecutionStatus::Invalid),
        ]);
        definition(gloas, operations)
    }

    #[test]
    fn rejected_on_import_pre_gloas() {
        rejected_on_import(false).run();
    }

    #[test]
    fn rejected_on_import_gloas() {
        rejected_on_import(true).run();
    }

    #[test]
    fn valid_after_invalid_ancestor_pre_gloas() {
        valid_after_invalid_ancestor(false).run();
    }

    #[test]
    fn valid_after_invalid_ancestor_gloas() {
        valid_after_invalid_ancestor(true).run();
    }
}
