use super::base_table::{self, InheritsFromTable, Table, TableLike};
use super::challenges_endpoints::{AllChallenges, AllEndpoints};
use super::extension_table::{ExtensionTable, Quotientable, QuotientableExtensionTable};
use super::table_column::HashTableColumn;
use crate::fri_domain::FriDomain;
use crate::state::DIGEST_LEN;
use crate::table::base_table::Extendable;
use crate::table::extension_table::Evaluable;
use crate::table::table_column::HashTableColumn::*;
use itertools::Itertools;
use std::ops::Mul;
use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::shared_math::mpolynomial::{Degree, MPolynomial};
use twenty_first::shared_math::x_field_element::XFieldElement;

pub const HASH_TABLE_PERMUTATION_ARGUMENTS_COUNT: usize = 0;
pub const HASH_TABLE_EVALUATION_ARGUMENT_COUNT: usize = 2;
pub const HASH_TABLE_INITIALS_COUNT: usize =
    HASH_TABLE_PERMUTATION_ARGUMENTS_COUNT + HASH_TABLE_EVALUATION_ARGUMENT_COUNT;

/// This is 18 because it combines: 12 stack_input_weights and 6 digest_output_weights.
pub const HASH_TABLE_EXTENSION_CHALLENGE_COUNT: usize = 18;

/// The number of constants used in each round of the permutation. Since Rescue Prime uses one round
/// constant per half-round, this number is twice the number of state elements.
pub const NUM_ROUND_CONSTANTS: usize = 32;

/// The number of rounds for Rescue Prime
pub const NUM_ROUNDS: usize = 8;

/// Capacity of Rescue Prime
pub const CAPACITY: usize = 4;

pub const BASE_WIDTH: usize = 49;
pub const FULL_WIDTH: usize = 53; // BASE_WIDTH + 2 * INITIALS_COUNT

#[derive(Debug, Clone)]
pub struct HashTable {
    inherited_table: Table<BFieldElement>,
}

impl InheritsFromTable<BFieldElement> for HashTable {
    fn inherited_table(&self) -> &Table<BFieldElement> {
        &self.inherited_table
    }

    fn mut_inherited_table(&mut self) -> &mut Table<BFieldElement> {
        &mut self.inherited_table
    }
}

#[derive(Debug, Clone)]
pub struct ExtHashTable {
    inherited_table: Table<XFieldElement>,
}

impl Evaluable for ExtHashTable {
    fn evaluate_consistency_constraints(
        &self,
        evaluation_point: &[XFieldElement],
    ) -> Vec<XFieldElement> {
        let round_number = evaluation_point[ROUNDNUMBER as usize];
        let state12 = evaluation_point[STATE12 as usize];
        let state13 = evaluation_point[STATE13 as usize];
        let state14 = evaluation_point[STATE14 as usize];
        let state15 = evaluation_point[STATE15 as usize];

        let round_number_is_not_1_or = (0..=8)
            .filter(|&r| r != 1)
            .map(|r| round_number.clone() - r.into())
            .fold(1.into(), XFieldElement::mul);

        vec![
            round_number_is_not_1_or * state12,
            round_number_is_not_1_or * state13,
            round_number_is_not_1_or * state14,
            round_number_is_not_1_or * state15,
        ]
    }
}

impl Quotientable for ExtHashTable {
    fn get_consistency_quotient_degree_bounds(&self) -> Vec<Degree> {
        vec![self.interpolant_degree() * (NUM_ROUNDS + 1) as Degree; CAPACITY]
    }
}

impl QuotientableExtensionTable for ExtHashTable {}

impl InheritsFromTable<XFieldElement> for ExtHashTable {
    fn inherited_table(&self) -> &Table<XFieldElement> {
        &self.inherited_table
    }

    fn mut_inherited_table(&mut self) -> &mut Table<XFieldElement> {
        &mut self.inherited_table
    }
}

impl TableLike<BFieldElement> for HashTable {}

impl Extendable for HashTable {
    fn get_padding_row(&self) -> Vec<BFieldElement> {
        vec![0.into(); BASE_WIDTH]
    }
}

impl TableLike<XFieldElement> for ExtHashTable {}

impl ExtHashTable {
    fn ext_boundary_constraints() -> Vec<MPolynomial<XFieldElement>> {
        let one = MPolynomial::from_constant(1.into(), FULL_WIDTH);
        let variables = MPolynomial::variables(FULL_WIDTH, 1.into());

        let round_number = variables[ROUNDNUMBER as usize].clone();
        let round_number_is_0_or_1 = round_number.clone() * (round_number - one);
        vec![round_number_is_0_or_1]
    }

    /// The implementation below is kept around for debugging purposes. This table evaluates the
    /// corresponding constraints directly by implementing the respective method in trait
    /// `Evaluable`, and does not use the polynomials below.
    fn ext_consistency_constraints() -> Vec<MPolynomial<XFieldElement>> {
        let constant = |c: u32| MPolynomial::from_constant(c.into(), FULL_WIDTH);
        let variables = MPolynomial::variables(FULL_WIDTH, 1.into());

        let round_number = variables[ROUNDNUMBER as usize].clone();
        let state12 = variables[STATE12 as usize].clone();
        let state13 = variables[STATE13 as usize].clone();
        let state14 = variables[STATE14 as usize].clone();
        let state15 = variables[STATE15 as usize].clone();

        let round_number_is_not_1_or = (0..=8)
            .filter(|&r| r != 1)
            .map(|r| round_number.clone() - constant(r))
            .fold(constant(1), MPolynomial::mul);

        vec![
            round_number_is_not_1_or.clone() * state12,
            round_number_is_not_1_or.clone() * state13,
            round_number_is_not_1_or.clone() * state14,
            round_number_is_not_1_or * state15,
        ]
    }

    fn ext_transition_constraints(
        _challenges: &HashTableChallenges,
    ) -> Vec<MPolynomial<XFieldElement>> {
        let constant = |c: u32| MPolynomial::from_constant(c.into(), 2 * FULL_WIDTH);
        let variables = MPolynomial::variables(2 * FULL_WIDTH, 1.into());

        let round_number = variables[ROUNDNUMBER as usize].clone();
        let round_number_next = variables[FULL_WIDTH + ROUNDNUMBER as usize].clone();

        let if_round_number_is_x_then_round_number_next_is_y = |x, y| {
            (0..=8)
                .filter(|&r| r != x)
                .map(|r| round_number.clone() - constant(r))
                .fold(round_number_next.clone() - constant(y), MPolynomial::mul)
        };

        vec![
            if_round_number_is_x_then_round_number_next_is_y(0, 0),
            if_round_number_is_x_then_round_number_next_is_y(1, 2),
            if_round_number_is_x_then_round_number_next_is_y(2, 3),
            if_round_number_is_x_then_round_number_next_is_y(3, 4),
            if_round_number_is_x_then_round_number_next_is_y(4, 5),
            if_round_number_is_x_then_round_number_next_is_y(5, 6),
            if_round_number_is_x_then_round_number_next_is_y(6, 7),
            if_round_number_is_x_then_round_number_next_is_y(7, 8),
            // if round number is 8, then round number next is 0 or 1
            if_round_number_is_x_then_round_number_next_is_y(8, 0)
                * (round_number_next - constant(1)),
            // todo: The remaining 7·16 = 112 constraints are left as an exercise to the reader.
        ]
    }

    fn ext_terminal_constraints(
        _challenges: &HashTableChallenges,
        _terminals: &HashTableEndpoints,
    ) -> Vec<MPolynomial<XFieldElement>> {
        vec![]
    }
}

impl HashTable {
    pub fn new_prover(num_trace_randomizers: usize, matrix: Vec<Vec<BFieldElement>>) -> Self {
        let unpadded_height = matrix.len();
        let padded_height = base_table::pad_height(unpadded_height);

        let omicron = base_table::derive_omicron(padded_height as u64);
        let inherited_table = Table::new(
            BASE_WIDTH,
            FULL_WIDTH,
            padded_height,
            num_trace_randomizers,
            omicron,
            matrix,
            "HashTable".to_string(),
        );

        Self { inherited_table }
    }

    pub fn codeword_table(&self, fri_domain: &FriDomain<BFieldElement>) -> Self {
        let base_columns = 0..self.base_width();
        let codewords = self.low_degree_extension(fri_domain, base_columns);

        let inherited_table = self.inherited_table.with_data(codewords);
        Self { inherited_table }
    }

    pub fn extend(
        &self,
        challenges: &HashTableChallenges,
        initials: &HashTableEndpoints,
    ) -> (ExtHashTable, HashTableEndpoints) {
        let mut from_processor_running_sum = initials.from_processor_eval_sum;
        let mut to_processor_running_sum = initials.to_processor_eval_sum;

        let mut extension_matrix: Vec<Vec<XFieldElement>> = Vec::with_capacity(self.data().len());
        for row in self.data().iter() {
            let mut extension_row = Vec::with_capacity(FULL_WIDTH);
            extension_row.extend(row.iter().map(|elem| elem.lift()));

            // Compress input values into single value (independent of round index)
            let state_for_input = [
                extension_row[HashTableColumn::STATE0 as usize],
                extension_row[HashTableColumn::STATE1 as usize],
                extension_row[HashTableColumn::STATE2 as usize],
                extension_row[HashTableColumn::STATE3 as usize],
                extension_row[HashTableColumn::STATE4 as usize],
                extension_row[HashTableColumn::STATE5 as usize],
                extension_row[HashTableColumn::STATE6 as usize],
                extension_row[HashTableColumn::STATE7 as usize],
                extension_row[HashTableColumn::STATE8 as usize],
                extension_row[HashTableColumn::STATE9 as usize],
                extension_row[HashTableColumn::STATE10 as usize],
                extension_row[HashTableColumn::STATE11 as usize],
            ];
            let compressed_state_for_input = state_for_input
                .iter()
                .zip(challenges.stack_input_weights.iter())
                .map(|(state, weight)| *weight * *state)
                .fold(XFieldElement::ring_zero(), |sum, summand| sum + summand);
            extension_row.push(compressed_state_for_input);

            // Add compressed input to running sum if round index marks beginning of hashing
            extension_row.push(from_processor_running_sum);
            if row[HashTableColumn::ROUNDNUMBER as usize].value() == 1 {
                from_processor_running_sum = from_processor_running_sum
                    * challenges.from_processor_eval_row_weight
                    + compressed_state_for_input;
            }

            // Compress digest values into single value (independent of round index)
            let state_for_output = [
                extension_row[HashTableColumn::STATE0 as usize],
                extension_row[HashTableColumn::STATE1 as usize],
                extension_row[HashTableColumn::STATE2 as usize],
                extension_row[HashTableColumn::STATE3 as usize],
                extension_row[HashTableColumn::STATE4 as usize],
                extension_row[HashTableColumn::STATE5 as usize],
            ];
            let compressed_state_for_output = state_for_output
                .iter()
                .zip(challenges.digest_output_weights.iter())
                .map(|(state, weight)| *weight * *state)
                .fold(XFieldElement::ring_zero(), |sum, summand| sum + summand);
            extension_row.push(compressed_state_for_output);

            // Add compressed digest to running sum if round index marks end of hashing
            extension_row.push(to_processor_running_sum);
            if row[HashTableColumn::ROUNDNUMBER as usize].value() == 8 {
                to_processor_running_sum = to_processor_running_sum
                    * challenges.to_processor_eval_row_weight
                    + compressed_state_for_output;
            }

            extension_matrix.push(extension_row);
        }

        let terminals = HashTableEndpoints {
            from_processor_eval_sum: from_processor_running_sum,
            to_processor_eval_sum: to_processor_running_sum,
        };

        let extension_table = self.extension(
            extension_matrix,
            ExtHashTable::ext_boundary_constraints(),
            ExtHashTable::ext_transition_constraints(challenges),
            ExtHashTable::ext_consistency_constraints(),
            ExtHashTable::ext_terminal_constraints(challenges, &terminals),
        );

        (
            ExtHashTable {
                inherited_table: extension_table,
            },
            terminals,
        )
    }

    pub fn for_verifier(
        num_trace_randomizers: usize,
        padded_height: usize,
        all_challenges: &AllChallenges,
        all_terminals: &AllEndpoints,
    ) -> ExtHashTable {
        let omicron = base_table::derive_omicron(padded_height as u64);
        let inherited_table = Table::new(
            BASE_WIDTH,
            FULL_WIDTH,
            padded_height,
            num_trace_randomizers,
            omicron,
            vec![],
            "ExtHashTable".to_string(),
        );
        let base_table = Self { inherited_table };
        let empty_matrix: Vec<Vec<XFieldElement>> = vec![];
        let extension_table = base_table.extension(
            empty_matrix,
            ExtHashTable::ext_boundary_constraints(),
            ExtHashTable::ext_transition_constraints(&all_challenges.hash_table_challenges),
            ExtHashTable::ext_consistency_constraints(),
            ExtHashTable::ext_terminal_constraints(
                &all_challenges.hash_table_challenges,
                &all_terminals.hash_table_endpoints,
            ),
        );

        ExtHashTable {
            inherited_table: extension_table,
        }
    }
}

impl ExtHashTable {
    pub fn with_padded_height(num_trace_randomizers: usize, padded_height: usize) -> Self {
        let matrix: Vec<Vec<XFieldElement>> = vec![];

        let omicron = base_table::derive_omicron(padded_height as u64);
        let inherited_table = Table::new(
            BASE_WIDTH,
            FULL_WIDTH,
            padded_height,
            num_trace_randomizers,
            omicron,
            matrix,
            "ExtHashTable".to_string(),
        );

        Self { inherited_table }
    }

    pub fn ext_codeword_table(
        &self,
        fri_domain: &FriDomain<XFieldElement>,
        base_codewords: &[Vec<BFieldElement>],
    ) -> Self {
        let ext_columns = self.base_width()..self.full_width();
        let ext_codewords = self.low_degree_extension(fri_domain, ext_columns);

        let lifted_base_codewords = base_codewords
            .iter()
            .map(|base_codeword| base_codeword.iter().map(|bfe| bfe.lift()).collect_vec())
            .collect_vec();
        let all_codewords = vec![lifted_base_codewords, ext_codewords].concat();
        assert_eq!(self.full_width(), all_codewords.len());

        let inherited_table = self.inherited_table.with_data(all_codewords);
        ExtHashTable { inherited_table }
    }
}

#[derive(Debug, Clone)]
pub struct HashTableChallenges {
    /// The weight that combines two consecutive rows in the
    /// permutation/evaluation column of the hash table.
    pub from_processor_eval_row_weight: XFieldElement,
    pub to_processor_eval_row_weight: XFieldElement,

    /// Weights for condensing part of a row into a single column. (Related to processor table.)
    pub stack_input_weights: [XFieldElement; 2 * DIGEST_LEN],
    pub digest_output_weights: [XFieldElement; DIGEST_LEN],
}

#[derive(Debug, Clone)]
pub struct HashTableEndpoints {
    /// Values randomly generated by the prover for zero-knowledge.
    pub from_processor_eval_sum: XFieldElement,
    pub to_processor_eval_sum: XFieldElement,
}

impl ExtensionTable for ExtHashTable {
    fn dynamic_boundary_constraints(&self) -> Vec<MPolynomial<XFieldElement>> {
        ExtHashTable::ext_boundary_constraints()
    }

    fn dynamic_transition_constraints(
        &self,
        challenges: &super::challenges_endpoints::AllChallenges,
    ) -> Vec<MPolynomial<XFieldElement>> {
        ExtHashTable::ext_transition_constraints(&challenges.hash_table_challenges)
    }

    fn dynamic_consistency_constraints(&self) -> Vec<MPolynomial<XFieldElement>> {
        ExtHashTable::ext_consistency_constraints()
    }

    fn dynamic_terminal_constraints(
        &self,
        challenges: &super::challenges_endpoints::AllChallenges,
        terminals: &super::challenges_endpoints::AllEndpoints,
    ) -> Vec<MPolynomial<XFieldElement>> {
        ExtHashTable::ext_terminal_constraints(
            &challenges.hash_table_challenges,
            &terminals.hash_table_endpoints,
        )
    }
}
