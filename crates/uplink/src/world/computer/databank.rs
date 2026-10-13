//! Port of source/world/computer/databank.h + source/world/computer/databank.cpp
//!
//! Data storage of a person, computer, message etc.: a list of files (`data`) plus a "FAT"
//! (`memory`) mapping each memory block to the index of the file stored in it.

use serde::{Deserialize, Serialize};

use crate::tosser::DArray;
use crate::world::generator::numbergenerator::NumberGenerator;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DataBank {
    data: DArray<Data>,  // All files
    memory: DArray<i32>, // indexes into data (ie FAT)
    pub formatted: bool, // Set if databank was recently wiped
}

impl DataBank {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_size(&mut self, newsize: i32) {
        self.memory.set_size(newsize);
    }

    pub fn get_size(&self) -> i32 {
        self.memory.size()
    }

    pub fn num_data_files(&self) -> i32 {
        self.data.num_used()
    }

    pub fn get_data_size(&self) -> i32 {
        self.data.size()
    }

    /// `PutData(newdata)`: stores at the first valid placement (does nothing if there is none).
    pub fn put_data(&mut self, newdata: Data) {
        let index = self.find_valid_placement(&newdata);
        if index != -1 {
            self.put_data_at(newdata, index);
        }
    }

    /// Inserts at a random position (uses NumberGenerator).
    pub fn insert_data(&mut self, newdata: Data) {
        if self.data.size() == 0 {
            self.put_data(newdata);
        } else {
            let insertindex = NumberGenerator::random_number(self.data.size());

            // Move the old data item from this location to the end

            if self.data.valid_index(insertindex) {
                self.data.set_size(self.data.size() + 1);
                let olddata = self
                    .data
                    .remove_data(insertindex)
                    .expect("DataBank::InsertData: olddata");
                let newindex = self.data.size() - 1;
                self.data.put_data_at(olddata, newindex);

                for i in 0..self.memory.size() {
                    if matches!(self.memory.get_data(i), Some(&v) if v == insertindex) {
                        self.memory.put_data_at(newindex, i);
                    }
                }
            }

            // Insert the new data item

            self.data.put_data_at(newdata.clone(), insertindex);

            let mut index = self.find_valid_placement(&newdata);

            if index == -1 {
                self.memory.set_size(self.memory.size() + newdata.size);
                index = self.find_valid_placement(&newdata);
            }

            assert!(index != -1, "DataBank::InsertData: no valid placement");

            for i in 0..newdata.size {
                self.memory.put_data_at(insertindex, index + i);
            }
        }
    }

    /// `PutData(newdata, memoryindex)`: overwrites.
    pub fn put_data_at(&mut self, newdata: Data, memoryindex: i32) {
        let size = newdata.size;
        let pos = self.data.put_data(newdata);
        for i in 0..size {
            self.memory.put_data_at(pos, memoryindex + i);
        }
    }

    /// Removes the file from data as well.
    pub fn remove_data(&mut self, memoryindex: i32) {
        if self.memory.valid_index(memoryindex) {
            let dataindex = self.get_data_index(memoryindex);
            self.remove_data_file(dataindex);
        }
    }

    /// Removes all references in memory as well.
    pub fn remove_data_file(&mut self, dataindex: i32) {
        if self.data.valid_index(dataindex) {
            // Delete the file

            self.data.remove_data(dataindex);

            // Delete any indexes to the file

            for i in 0..self.memory.size() {
                if matches!(self.memory.get_data(i), Some(&v) if v == dataindex) {
                    self.memory.remove_data(i);
                }
            }

            // If that was the last file, then this databank has been formatted

            if self.num_data_files() == 0 {
                self.formatted = true;
            }
        }
    }

    /// 0 = yes, 1 = will overwrite, 2 = no
    pub fn is_valid_placement(&self, newdata: &Data, memoryindex: i32) -> i32 {
        if memoryindex < 0 {
            return 2;
        }

        for i in memoryindex..memoryindex + newdata.size {
            if i >= self.memory.size() || i < 0 {
                return 2; // Not valid
            } else if self.memory.valid_index(i) {
                return 1; // Will overwrite
            }
        }

        0 // No conflicts
    }

    /// -1 = failure
    pub fn find_valid_placement(&self, newdata: &Data) -> i32 {
        for i in 0..self.memory.size() {
            if self.is_valid_placement(newdata, i) == 0 {
                return i;
            }
        }
        -1
    }

    /// `GetData(memoryindex)`
    pub fn get_data(&self, memoryindex: i32) -> Option<&Data> {
        let dataindex = self.get_data_index(memoryindex);
        self.data.get_data(dataindex)
    }

    pub fn get_data_mut(&mut self, memoryindex: i32) -> Option<&mut Data> {
        let dataindex = self.get_data_index(memoryindex);
        self.data.get_data_mut(dataindex)
    }

    /// `GetDataFile(dataindex)`
    pub fn get_data_file(&self, dataindex: i32) -> Option<&Data> {
        self.data.get_data(dataindex)
    }

    pub fn get_data_file_mut(&mut self, dataindex: i32) -> Option<&mut Data> {
        self.data.get_data_mut(dataindex)
    }

    /// `GetData(char *title)`: first file with this title.
    pub fn get_data_by_title(&self, title: &str) -> Option<&Data> {
        self.data.values().find(|d| d.title == title)
    }

    pub fn get_data_by_title_mut(&mut self, title: &str) -> Option<&mut Data> {
        self.data
            .iter_mut()
            .map(|(_, d)| d)
            .find(|d| d.title == title)
    }

    /// Returns index of data in this memory block, or -1.
    pub fn get_data_index(&self, memoryindex: i32) -> i32 {
        match self.memory.get_data(memoryindex) {
            Some(&dataindex) if self.data.valid_index(dataindex) => dataindex,
            _ => -1,
        }
    }

    /// Finds first memory index pointing to the data, or -1.
    pub fn get_memory_index(&self, dataindex: i32) -> i32 {
        self.memory
            .iter()
            .find(|(_, &d)| d == dataindex)
            .map(|(i, _)| i)
            .unwrap_or(-1)
    }

    /// Wipes everything (keeps the memory size).
    pub fn format(&mut self) {
        let oldmemsize = self.memory.size();
        self.data.empty();
        self.memory.empty();
        self.memory.set_size(oldmemsize);
        self.formatted = true;
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}

// ============================================================================

/// `SIZE_DATA_TITLE`: size of the C++ title buffer (titles must be shorter).
const SIZE_DATA_TITLE: usize = 64;

pub const DATATYPE_NONE: i32 = 0;
pub const DATATYPE_DATA: i32 = 1;
pub const DATATYPE_PROGRAM: i32 = 2;

pub const SOFTWARETYPE_NONE: i32 = 0;
pub const SOFTWARETYPE_FILEUTIL: i32 = 1;
pub const SOFTWARETYPE_HWDRIVER: i32 = 2;
pub const SOFTWARETYPE_SECURITY: i32 = 3;
pub const SOFTWARETYPE_CRACKERS: i32 = 4;
pub const SOFTWARETYPE_BYPASSER: i32 = 5;
pub const SOFTWARETYPE_LANTOOL: i32 = 6;
pub const SOFTWARETYPE_HUDUPGRADE: i32 = 9;
pub const SOFTWARETYPE_OTHER: i32 = 10;

/// A file. The C++ copy constructor `Data(Data *copyme)` is `Clone`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Data {
    pub title: String,
    pub type_: i32,
    pub size: i32,
    pub encrypted: i32,  // 0 = not encrypted, >=1 = encrypted
    pub compressed: i32, // 0 = not compressed, >=1 = compressed
    pub version: f32,
    pub software_type: i32, // C++ `softwareTYPE`
}

impl Default for Data {
    fn default() -> Self {
        Data {
            title: "Empty".to_string(),
            type_: DATATYPE_NONE,
            size: 0,
            encrypted: 0,
            compressed: 0,
            version: 1.0,
            software_type: SOFTWARETYPE_NONE,
        }
    }
}

impl Data {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_title(&mut self, newtitle: &str) {
        assert!(newtitle.len() < SIZE_DATA_TITLE, "Data::SetTitle: title too long");
        self.title = newtitle.to_string();
    }

    /// C++ defaults: encrypted 0, compressed 0, version 1.0, softwareTYPE SOFTWARETYPE_NONE.
    pub fn set_details(
        &mut self,
        new_type: i32,
        newsize: i32,
        newencrypted: i32,
        newcompressed: i32,
        newversion: f32,
        new_software_type: i32,
    ) {
        self.type_ = new_type;
        self.size = newsize;
        self.encrypted = newencrypted;
        self.compressed = newcompressed;
        self.version = newversion;
        self.software_type = new_software_type;
    }

    /// Does nothing in the C++ either.
    pub fn update(&mut self) {}
}
