use compression_core::{CompressedImage, CompressionMetadata, ImageCompressor};
use image::RgbImage;
use std::collections::HashMap;

#[derive(Clone)]
enum HuffmanNode {
    Internal {
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>,
        freq: u32,
    },
    Leaf {
        value: (u8, u8, u8),
        freq: u32,
    },
}

impl HuffmanNode {
    fn freq(&self) -> u32 {
        match self {
            HuffmanNode::Internal { freq, .. } => *freq,
            HuffmanNode::Leaf { freq, .. } => *freq,
        }
    }
}

pub struct HuffmanCompressor {}

impl HuffmanCompressor {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImageCompressor for HuffmanCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let width = image.width();
        let height = image.height();
        let raw: &[u8] = image.as_raw();

        let frequencies_map = get_color_frequencies(image);
        let sorted_frequency_map = sort_color_frequencies(frequencies_map);
        let mut nodes = convert_to_nodes(sorted_frequency_map);

        while nodes.len() > 1 {
            let right = nodes.pop().unwrap();
            let left = nodes.pop().unwrap();

            let node = HuffmanNode::Internal {
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
                freq: left.freq() + right.freq(),
            };

            let pos = nodes
                .binary_search_by(|n| node.freq().cmp(&n.freq()))
                .unwrap_or_else(|x| x);
            nodes.insert(pos, node);
        }

        let mut codes = HashMap::new();
        explore_node(&nodes[0], 0, 0, &mut codes);

        let mut writer = BitWriter::new();
        let mut i: usize = 0;
        while i < raw.len() {
            let pixel = (raw[i], raw[i + 1], raw[i + 2]);
            let (code, len) = codes.get(&pixel).unwrap();

            writer.write_code(*code, *len);
            i += 3;
        }

        let data = writer.bytes;

        let flipped: HashMap<(u32, u8), (u8, u8, u8)> =
            codes.iter().map(|(k, v)| (*v, *k)).collect();

        Ok(CompressedImage {
            data,
            width,
            height,
            metadata: CompressionMetadata::Huffman {
                freq_table: flipped,
            },
        })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        let mut reader = BitReader::new(compressed.data.clone());
        let map = match compressed.metadata.clone() {
            CompressionMetadata::Huffman { freq_table } => Some(freq_table),
            _ => None,
        }
        .unwrap();
        let mut raw = Vec::with_capacity(
            (compressed.width * compressed.height * 3)
                .try_into()
                .unwrap(),
        );

        let mut i: usize = 0;
        while i < (compressed.width * compressed.height).try_into().unwrap() {
            let mut current_code: u32 = 0;
            let mut current_len: u8 = 0;

            while !map.contains_key(&(current_code, current_len)) {
                let bit = reader.read_bit().unwrap();
                current_code = (current_code << 1) | bit as u32;
                current_len += 1;
            }

            let p = map.get(&(current_code, current_len)).unwrap();
            raw.extend_from_slice(&[p.0, p.1, p.2]);

            i += 1;
        }

        RgbImage::from_raw(compressed.width, compressed.height, raw)
            .ok_or_else(|| "Buffer too small for image dimensions".to_string())
    }
}

fn get_color_frequencies(image: &RgbImage) -> HashMap<(u8, u8, u8), u32> {
    let mut map: HashMap<(u8, u8, u8), u32> = HashMap::new();
    image.enumerate_pixels().for_each(|p| {
        *map.entry(p.2.0.into()).or_insert(0) += 1;
    });
    map
}

fn sort_color_frequencies(map: HashMap<(u8, u8, u8), u32>) -> Vec<((u8, u8, u8), u32)> {
    let mut vec: Vec<_> = map.into_iter().collect();
    vec.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    vec
}

fn convert_to_nodes(map: Vec<((u8, u8, u8), u32)>) -> Vec<HuffmanNode> {
    let nodes = map
        .iter()
        .map(|m| HuffmanNode::Leaf {
            value: m.0,
            freq: m.1,
        })
        .collect();
    nodes
}

fn explore_node(
    node: &HuffmanNode,
    code: u32,
    depth: u8,
    map: &mut HashMap<(u8, u8, u8), (u32, u8)>,
) {
    match node {
        HuffmanNode::Leaf { value, .. } => {
            map.insert(*value, (code, depth));
        }
        HuffmanNode::Internal { left, right, .. } => {
            explore_node(left, code << 1, depth + 1, map);
            explore_node(right, (code << 1) | 1, depth + 1, map);
        }
    }
}

struct BitWriter {
    pub bytes: Vec<u8>,
    bit_pos: u8,
}

impl BitWriter {
    fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            bit_pos: 0,
        }
    }

    fn write_bit(&mut self, bit: bool) {
        if self.bit_pos == 0 {
            self.bytes.push(0);
        }
        if bit {
            let last = self.bytes.last_mut().unwrap();
            *last |= 1 << (7 - self.bit_pos);
        }
        self.bit_pos = (self.bit_pos + 1) % 8;
    }

    fn write_code(&mut self, code: u32, len: u8) {
        for i in (0..len).rev() {
            self.write_bit((code >> i) & 1 == 1);
        }
    }
}

struct BitReader {
    bytes: Vec<u8>,
    byte_pos: usize,
    bit_pos: u8,
}

impl BitReader {
    fn new(bytes: Vec<u8>) -> Self {
        BitReader {
            bytes,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Option<bool> {
        if self.byte_pos >= self.bytes.len() {
            return None;
        }
        let bit = (self.bytes[self.byte_pos] >> (7 - self.bit_pos)) & 1 == 1;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Some(bit)
    }
}
