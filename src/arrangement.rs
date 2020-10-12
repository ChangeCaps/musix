use crate::{audio::AudioSourceID, AudioBlockID, widgets::arrangement::*};
use druid::*;
use std::{collections::HashMap, ops::Range, sync::Arc};

#[derive(Clone, Data, Lens)]
pub struct Arrangement {
    pub tracks: Arc<Vec<Track>>,
    pub beats: usize,
}

impl Arrangement {
    pub fn new() -> Self {
        Self {
            tracks: Arc::new(vec![Track::new()]),
            beats: 4,
        }
    }

    pub fn add_track(&mut self) {
        Arc::make_mut(&mut self.tracks).push(Track::new())
    }

    pub fn remove_track(&mut self, idx: usize) {
        Arc::make_mut(&mut self.tracks).remove(idx);
    }
}

// A battle was fought here, it was long, it was tough, but in the end, the world was better for
// it.
//      -Hjalte Nannestad, during the rewrite of the track struct of October 2020.
#[derive(Clone, Default)]
pub struct Track {
    pub beats: HashMap<usize, usize>,
    pub blocks: Vec<Block>,
}

impl Track {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_selection(&self, beat: usize) -> Option<Selection> {
        let selected = self.beats.get(&beat);
        let prev_selected = self.beats.get(&(beat - 1));
        if selected.is_some() && self.blocks[*selected.unwrap()].bounds.start == beat {
            Some(Selection::Some(beat, *selected.unwrap()))
        } else if prev_selected.is_some() && self.blocks[*prev_selected.unwrap()].bounds.end == beat
        {
            Some(Selection::Some(beat, *prev_selected.unwrap()))
        } else {
            Some(Selection::None(beat))
        }
    }

    pub fn calculate_beats(&mut self) {
        self.beats.clear();

        let mut place = 0;

        for (block_index, block) in self.blocks.iter().enumerate() {
            for i in place..block.bounds.end {
                self.beats.insert(i, block_index);
            }

            place = block.bounds.end;
        }
    }

    pub fn get_block(&self, beat: usize) -> Option<&Block> {
        if let Some(beat_index) = self.beats.get(&beat) {
            if beat >= self.blocks[*beat_index].bounds.start {
                Some(&self.blocks[*beat_index])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_space(&self, block_index: usize) -> Range<usize> {
        let start = if let Some(block) = self.blocks.get(block_index - 1) {
            block.bounds.end
        } else {
            0
        };

        let end = if let Some(block) = self.blocks.get(block_index + 1) {
            block.bounds.start
        } else {
            usize::MAX
        };

        start..end
    }

    pub fn move_block_bound(&mut self, block_index: usize, bound: usize, target: usize) -> bool {
        let space = self.get_space(block_index);

        match bound {
            b if b == self.blocks[block_index].bounds.start => {
                if target >= space.start && target < self.blocks[block_index].bounds.end {
                    self.blocks[block_index].bounds.start = target;
                    self.calculate_beats();
                    true
                } else {
                    false
                }
            }
            b if b == self.blocks[block_index].bounds.end => {
                if target <= space.end && target > self.blocks[block_index].bounds.start {
                    self.blocks[block_index].bounds.end = target;
                    self.calculate_beats();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn add_block(&mut self, block: Block) -> Option<usize> {
        let start_index = self.beats.get(&block.bounds.start);
        let end_index = self.beats.get(&block.bounds.end);

        if start_index == end_index {
            if let Some(index) = start_index {
                let index = *index;

                if block.bounds.end < self.blocks[index].bounds.start {
                    self.blocks.insert(index, block);
                    self.calculate_beats();

                    Some(index)
                } else {
                    None
                }
            } else {
                let index = self.blocks.len();
                self.blocks.push(block);
                self.calculate_beats();

                Some(index)
            }
        } else {
            None
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Block {
    pub bounds: Range<usize>,
    pub audio_id: AudioSourceID,
    pub audio_block_id: AudioBlockID,
}

impl Block {
    pub fn new(bounds: Range<usize>, audio_id: AudioSourceID, audio_block_id: AudioBlockID) -> Self {
        Self {
            bounds,
            audio_id,
            audio_block_id,
        }
    }
}


