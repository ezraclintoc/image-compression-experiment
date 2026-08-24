use compression_core::{CompressedImage, CompressionMetadata, ImageCompressor};
use image::RgbImage;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

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

impl PartialEq for HuffmanNode {
    fn eq(&self, other: &Self) -> bool {
        self.freq() == other.freq()
    }
}
impl Eq for HuffmanNode {}
impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.freq().cmp(&other.freq())
    }
}


impl ImageCompressor for HuffmanCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let width = image.width();
        let height = image.height();
        let raw: &[u8] = image.as_raw();

        // Min-heap of leaves keyed on frequency; repeatedly merge the two
        // least-frequent nodes until a single tree (the root) remains.
        let mut heap: BinaryHeap<Reverse<HuffmanNode>> = get_color_frequencies(image)
            .into_iter()
            .map(|(value, freq)| Reverse(HuffmanNode::Leaf { value, freq }))
            .collect();

        while heap.len() > 1 {
            let Reverse(left) = heap.pop().unwrap();
            let Reverse(right) = heap.pop().unwrap();

            let freq = left.freq() + right.freq();
            heap.push(Reverse(HuffmanNode::Internal {
                left: Box::new(left),
                right: Box::new(right),
                freq,
            }));
        }

        // An empty image leaves an empty heap and no codes; the encode loop
        // below then runs zero times, producing empty data.
        let mut codes = HashMap::new();
        if let Some(Reverse(root)) = heap.pop() {
            explore_node(&root, 0, 0, &mut codes);
        }

        let mut writer = BitWriter::new();
        for pixel in raw.chunks_exact(3) {
            let (code, len) = codes.get(&(pixel[0], pixel[1], pixel[2])).unwrap();
            writer.write_code(*code, *len);
        }

        let data = writer.finish();

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

/// Buffered bit writer: bits accumulate in a 64-bit register and are flushed a
/// whole byte at a time, instead of touching the output `Vec` once per bit.
/// Bits are packed MSB-first (matching `BitReader`).
struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,   // pending bits, held in the low `nbits` bits of the register
    nbits: u32, // valid buffered bits; always < 8 between `write_code` calls
}

impl BitWriter {
    fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Append the low `len` bits of `code` (which must be < 2^len), MSB-first.
    /// With `nbits < 8` on entry and `len <= 32`, the register never overflows.
    fn write_code(&mut self, code: u32, len: u8) {
        let len = len as u32;
        self.acc = (self.acc << len) | code as u64;
        self.nbits += len;
        while self.nbits >= 8 {
            self.nbits -= 8;
            self.bytes.push((self.acc >> self.nbits) as u8);
        }
    }

    /// Flush a trailing partial byte (zero-padded on the right) and return the
    /// finished buffer.
    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push((self.acc << (8 - self.nbits)) as u8);
            self.nbits = 0;
        }
        self.bytes
    }
}

/// Buffered bit reader mirroring `BitWriter`: refills a 64-bit register from the
/// byte buffer and serves bits from it, avoiding the per-bit bounds check and
/// `% 8` of a naive reader.
struct BitReader {
    bytes: Vec<u8>,
    byte_pos: usize,
    acc: u64,   // buffered bits; the next bit is the MSB of the low `nbits` bits
    nbits: u32,
}

impl BitReader {
    fn new(bytes: Vec<u8>) -> Self {
        BitReader {
            bytes,
            byte_pos: 0,
            acc: 0,
            nbits: 0,
        }
    }

    fn read_bit(&mut self) -> Option<bool> {
        if self.nbits == 0 {
            if self.byte_pos >= self.bytes.len() {
                return None;
            }
            // Pull in up to 8 bytes at once. Already-consumed bits sit above
            // `nbits` and are never read, so the register needn't be cleared.
            let mut count = 0;
            while count < 8 && self.byte_pos < self.bytes.len() {
                self.acc = (self.acc << 8) | self.bytes[self.byte_pos] as u64;
                self.byte_pos += 1;
                count += 1;
            }
            self.nbits = count * 8;
        }
        self.nbits -= 1;
        Some((self.acc >> self.nbits) & 1 == 1)
    }
}
