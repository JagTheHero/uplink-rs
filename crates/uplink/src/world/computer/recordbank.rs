//! Port of source/world/computer/recordbank.h + source/world/computer/recordbank.cpp
//!
//! A bank of computer records, i.e. a database.
//!
//! Query syntax for `get_records` and friends (conditions separated by " ; ", every operator
//! surrounded by spaces, e.g. `"Name = Bob ; Password + ab"`):
//! `=` equals, `!` not equal, `+` contains, `-` does not contain.

use serde::{Deserialize, Serialize};

use crate::tosser::BTree;
use crate::world::generator::numbergenerator::NumberGenerator;

pub const RECORDBANK_NAME: &str = "Name";
pub const RECORDBANK_PASSWORD: &str = "Password";
pub const RECORDBANK_ACCNO: &str = "Account Number";
pub const RECORDBANK_SECURITY: &str = "Security";
pub const RECORDBANK_ADMIN: &str = "admin";
pub const RECORDBANK_READWRITE: &str = "readwrite";
pub const RECORDBANK_READONLY: &str = "readonly";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordBank {
    pub records: Vec<Record>,
}

impl RecordBank {
    pub fn new() -> Self {
        Self::default()
    }

    // Data access functions

    pub fn add_record(&mut self, newrecord: Record) {
        self.records.push(newrecord);
    }

    /// Inserts before the first record whose `sortfield` value sorts after the new record's
    /// (strcmp order), else at the end. Pass `RECORDBANK_NAME` for the C++ default.
    pub fn add_record_sorted(&mut self, newrecord: Record, sortfield: &str) {
        let newvalue = newrecord
            .get_field(sortfield)
            .expect("RecordBank::AddRecordSorted, new record has no sort field")
            .to_string();

        // Insert before the first record whose field sorts after the new value; else at the end.
        let mut index = self.records.len();
        for (i, rec) in self.records.iter().enumerate() {
            let fieldvalue = rec
                .get_field(sortfield)
                .expect("RecordBank::AddRecordSorted, record has no sort field");
            // strcmp(fieldvalue, newvalue) > 0
            if fieldvalue > newvalue.as_str() {
                index = i;
                break;
            }
        }
        self.records.insert(index, newrecord);
    }

    /// `GetRecord(int index)`: None if not found.
    pub fn get_record(&self, index: i32) -> Option<&Record> {
        if index < 0 {
            return None;
        }
        self.records.get(index as usize)
    }

    pub fn get_record_mut(&mut self, index: i32) -> Option<&mut Record> {
        if index < 0 {
            return None;
        }
        self.records.get_mut(index as usize)
    }

    /// `GetRecord(char *query)`: assumes there is only 1 match (returns the first otherwise).
    pub fn get_record_query(&self, query: &str) -> Option<&Record> {
        let matches = self.matching_indices(query)?;
        if matches.len() > 1 {
            println!("RecordBank::GetRecord, found more than 1 match, returning the first");
        }
        self.records.get(matches[0])
    }

    pub fn get_record_query_mut(&mut self, query: &str) -> Option<&mut Record> {
        let matches = self.matching_indices(query)?;
        if matches.len() > 1 {
            println!("RecordBank::GetRecord, found more than 1 match, returning the first");
        }
        self.records.get_mut(matches[0])
    }

    /// PORT: C++ returns NULL when nothing matches; here `None` (never `Some(empty)`).
    pub fn get_records(&self, query: &str) -> Option<Vec<&Record>> {
        let matches = self.matching_indices(query)?;
        Some(matches.into_iter().map(|i| &self.records[i]).collect())
    }

    /// Picks a random matching record (uses NumberGenerator).
    pub fn get_random_record(&self, query: &str) -> Option<&Record> {
        let matches = self.matching_indices(query)?;
        let index = NumberGenerator::random_number(matches.len() as i32) as usize;
        self.records.get(matches[index])
    }

    /// Indices of the records matching every condition of `query`, in record order.
    /// `None` when nothing matches (the C++ GetRecords returns NULL then). The query is
    /// parsed before the records are tested, so an invalid query aborts even with no records.
    fn matching_indices(&self, query: &str) -> Option<Vec<usize>> {
        // Split the query on ';'. Each condition loses the space that surrounds the ';'
        // (C++ overwrites that space with a terminator and skips the "; " after it).
        let pieces: Vec<&str> = query.split(';').collect();
        let numpieces = pieces.len();
        let conditions: Vec<&str> = pieces
            .iter()
            .enumerate()
            .map(|(i, piece)| {
                let s = if i > 0 { piece.get(1..).unwrap_or("") } else { piece };
                if i + 1 < numpieces {
                    s.get(..s.len().saturating_sub(1)).unwrap_or("")
                } else {
                    s
                }
            })
            .collect();

        // Parse each condition into field name, operation and required value. The op is
        // the first '=' in the condition, else '!', else '+', else '-' (C++ strchr order).
        let mut fields: Vec<&str> = Vec::with_capacity(conditions.len());
        let mut ops: Vec<u8> = Vec::with_capacity(conditions.len());
        let mut values: Vec<&str> = Vec::with_capacity(conditions.len());
        for cond in &conditions {
            let oplocation = cond
                .find('=')
                .or_else(|| cond.find('!'))
                .or_else(|| cond.find('+'))
                .or_else(|| cond.find('-'))
                .unwrap_or_else(|| panic!("RecordBank::GetRecords, invalid query"));
            // The op is surrounded by spaces: the field ends before the space, the value starts after it.
            fields.push(cond.get(..oplocation.saturating_sub(1)).unwrap_or(""));
            ops.push(cond.as_bytes()[oplocation]);
            values.push(cond.get(oplocation + 2..).unwrap_or(""));
        }

        // Test the conditions on each record; a record matches when every condition holds.
        let mut results = Vec::new();
        for (ri, rec) in self.records.iter().enumerate() {
            let mut nummatches = 0;
            for i in 0..conditions.len() {
                let reqfield = fields[i];
                let reqvalue = values[i];
                // A condition on a field the record lacks never counts as a match.
                if let Some(thisvalue) = rec.get_field(reqfield) {
                    match ops[i] {
                        b'=' => {
                            if thisvalue == reqvalue {
                                nummatches += 1;
                            }
                        }
                        b'!' => {
                            if thisvalue != reqvalue {
                                nummatches += 1;
                            }
                        }
                        b'+' => {
                            if thisvalue.contains(reqvalue) {
                                nummatches += 1;
                            }
                        }
                        b'-' => {
                            if !thisvalue.contains(reqvalue) {
                                nummatches += 1;
                            }
                        }
                        _ => panic!("RecordBank::GetRecord, unrecognised op code"),
                    }
                }
            }

            if nummatches == conditions.len() {
                results.push(ri);
            }
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    pub fn get_record_from_name(&self, name: &str) -> Option<&Record> {
        self.get_record_query(&format!("{} = {}", RECORDBANK_NAME, name))
    }

    pub fn get_record_from_name_mut(&mut self, name: &str) -> Option<&mut Record> {
        self.get_record_query_mut(&format!("{} = {}", RECORDBANK_NAME, name))
    }

    pub fn get_record_from_name_password(&self, name: &str, password: &str) -> Option<&Record> {
        self.get_record_query(&format!(
            "{} = {} ; {} = {}",
            RECORDBANK_NAME, name, RECORDBANK_PASSWORD, password
        ))
    }

    pub fn get_record_from_account_number(&self, acc_no: &str) -> Option<&Record> {
        self.get_record_query(&format!("{} = {}", RECORDBANK_ACCNO, acc_no))
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}

// ============================================================================

/// A computer record, e.g. a criminal record, a bank account, an uplink account.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub fields: BTree<String>,
}

impl Record {
    pub fn new() -> Self {
        Self::default()
    }

    /// An existing name gets a second (shadowed) entry: GetField keeps returning the first value.
    pub fn add_field(&mut self, name: &str, value: &str) {
        self.fields.put_data(name, value.to_string());
    }

    pub fn add_field_int(&mut self, name: &str, value: i32) {
        self.add_field(name, &value.to_string());
    }

    /// Creates the field (with a warning) when it does not exist yet.
    pub fn change_field(&mut self, name: &str, newvalue: &str) {
        if let Some(data) = self.fields.get_mut(name) {
            *data = newvalue.to_string();
        } else {
            eprintln!(
                "Record::ChangeField, WARNING : field {} not found (created instead)",
                name
            );
            self.add_field(name, newvalue);
        }
    }

    pub fn change_field_int(&mut self, name: &str, newvalue: i32) {
        self.change_field(name, &newvalue.to_string());
    }

    pub fn get_field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(|s| s.as_str())
    }

    /// tosser BTree::RemoveData, with its quirks (see `BTree::remove_data`).
    pub fn delete_field(&mut self, name: &str) {
        self.fields.remove_data(name);
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}
