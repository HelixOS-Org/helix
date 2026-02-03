//! SPIR-V Binary Encoding/Decoding
//!
//! Handles encoding SPIR-V instructions to binary format and decoding binary back to instructions.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
use core::mem;

use crate::instruction::*;
use crate::opcode::Opcode;
use crate::{SpirVError, SpirVResult, SPIRV_MAGIC};

/// Binary encoder for SPIR-V
#[derive(Debug, Default)]
pub struct BinaryEncoder {
    /// Output words
    words: Vec<u32>,
}

impl BinaryEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            words: Vec::with_capacity(capacity),
        }
    }

    /// Get the encoded words
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// Take the encoded words
    pub fn take_words(self) -> Vec<u32> {
        self.words
    }

    /// Get binary data
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.words.len() * 4);
        for word in &self.words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Encode header
    pub fn encode_header(&mut self, version: u32, generator: u32, bound: Id, schema: u32) {
        self.words.push(SPIRV_MAGIC);
        self.words.push(version);
        self.words.push(generator);
        self.words.push(bound);
        self.words.push(schema);
    }

    /// Encode an instruction
    pub fn encode_instruction(&mut self, inst: &Instruction) {
        let encoded = inst.encode();
        self.words.extend(encoded);
    }

    /// Encode a word
    pub fn encode_word(&mut self, word: u32) {
        self.words.push(word);
    }

    /// Encode a string
    pub fn encode_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let mut word = 0u32;
        for (i, &b) in bytes.iter().enumerate() {
            word |= (b as u32) << ((i % 4) * 8);
            if i % 4 == 3 {
                self.words.push(word);
                word = 0;
            }
        }
        self.words.push(word);
    }

    /// Encode multiple instructions
    pub fn encode_instructions(&mut self, instructions: &[Instruction]) {
        for inst in instructions {
            self.encode_instruction(inst);
        }
    }

    /// Current position in words
    pub fn position(&self) -> usize {
        self.words.len()
    }

    /// Clear the encoder
    pub fn clear(&mut self) {
        self.words.clear();
    }
}

/// Binary decoder for SPIR-V
#[derive(Debug)]
pub struct BinaryDecoder<'a> {
    /// Input words
    words: &'a [u32],
    /// Current position
    position: usize,
}

impl<'a> BinaryDecoder<'a> {
    /// Create a new decoder from words
    pub fn new(words: &'a [u32]) -> Self {
        Self { words, position: 0 }
    }

    /// Create from bytes
    pub fn from_bytes(bytes: &'a [u8]) -> SpirVResult<Self> {
        if bytes.len() % 4 != 0 {
            return Err(SpirVError::Validation("Invalid binary length".into()));
        }

        // Safety: we're casting aligned u8 slice to u32 slice
        let words =
            unsafe { core::slice::from_raw_parts(bytes.as_ptr() as *const u32, bytes.len() / 4) };

        Ok(Self::new(words))
    }

    /// Remaining words
    pub fn remaining(&self) -> usize {
        self.words.len() - self.position
    }

    /// Check if at end
    pub fn is_empty(&self) -> bool {
        self.position >= self.words.len()
    }

    /// Current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Read a word
    pub fn read_word(&mut self) -> SpirVResult<u32> {
        if self.position >= self.words.len() {
            return Err(SpirVError::Validation("Unexpected end of binary".into()));
        }
        let word = self.words[self.position];
        self.position += 1;
        Ok(word)
    }

    /// Peek a word
    pub fn peek_word(&self) -> SpirVResult<u32> {
        if self.position >= self.words.len() {
            return Err(SpirVError::Validation("Unexpected end of binary".into()));
        }
        Ok(self.words[self.position])
    }

    /// Read multiple words
    pub fn read_words(&mut self, count: usize) -> SpirVResult<&'a [u32]> {
        if self.position + count > self.words.len() {
            return Err(SpirVError::Validation("Unexpected end of binary".into()));
        }
        let slice = &self.words[self.position..self.position + count];
        self.position += count;
        Ok(slice)
    }

    /// Read a string
    pub fn read_string(&mut self) -> SpirVResult<String> {
        let start = self.position;
        let mut found_null = false;

        // Find null terminator
        'outer: while self.position < self.words.len() {
            let word = self.words[self.position];
            self.position += 1;

            for i in 0..4 {
                if ((word >> (i * 8)) & 0xFF) == 0 {
                    found_null = true;
                    break 'outer;
                }
            }
        }

        if !found_null {
            return Err(SpirVError::Validation("Unterminated string".into()));
        }

        // Extract string bytes
        let mut bytes = Vec::new();
        for word in &self.words[start..self.position] {
            for i in 0..4 {
                let b = ((word >> (i * 8)) & 0xFF) as u8;
                if b == 0 {
                    break;
                }
                bytes.push(b);
            }
        }

        String::from_utf8(bytes).map_err(|_| SpirVError::Validation("Invalid UTF-8 string".into()))
    }

    /// Decode header
    pub fn decode_header(&mut self) -> SpirVResult<Header> {
        let magic = self.read_word()?;
        if magic != SPIRV_MAGIC {
            return Err(SpirVError::Validation("Invalid SPIR-V magic number".into()));
        }

        let version = self.read_word()?;
        let generator = self.read_word()?;
        let bound = self.read_word()?;
        let schema = self.read_word()?;

        Ok(Header {
            version,
            generator,
            bound,
            schema,
        })
    }

    /// Decode next instruction
    pub fn decode_instruction(&mut self) -> SpirVResult<Instruction> {
        let first_word = self.read_word()?;
        let word_count = (first_word >> 16) as usize;
        let opcode_raw = (first_word & 0xFFFF) as u16;

        let opcode = Opcode::from_u16(opcode_raw).ok_or_else(|| {
            SpirVError::Validation(format!("Unknown opcode: {}", opcode_raw).into())
        })?;

        // Read remaining words
        let operand_words = if word_count > 1 {
            self.read_words(word_count - 1)?
        } else {
            &[]
        };

        // Parse based on opcode class
        self.parse_instruction(opcode, operand_words)
    }

    /// Parse instruction operands
    fn parse_instruction(&self, opcode: Opcode, words: &[u32]) -> SpirVResult<Instruction> {
        let mut inst = Instruction::new(opcode);
        let mut pos = 0;

        // Handle instructions with result type and/or result
        match opcode {
            // Instructions with both result type and result
            Opcode::OpTypeVoid
            | Opcode::OpTypeBool
            | Opcode::OpTypeInt
            | Opcode::OpTypeFloat
            | Opcode::OpTypeVector
            | Opcode::OpTypeMatrix
            | Opcode::OpTypeImage
            | Opcode::OpTypeSampler
            | Opcode::OpTypeSampledImage
            | Opcode::OpTypeArray
            | Opcode::OpTypeRuntimeArray
            | Opcode::OpTypeStruct
            | Opcode::OpTypePointer
            | Opcode::OpTypeFunction
            | Opcode::OpLabel => {
                // Only result, no result type
                if pos < words.len() {
                    inst.result = Some(words[pos]);
                    pos += 1;
                }
            },
            Opcode::OpConstant
            | Opcode::OpConstantComposite
            | Opcode::OpConstantTrue
            | Opcode::OpConstantFalse
            | Opcode::OpSpecConstant
            | Opcode::OpSpecConstantTrue
            | Opcode::OpSpecConstantFalse
            | Opcode::OpVariable
            | Opcode::OpLoad
            | Opcode::OpAccessChain
            | Opcode::OpInBoundsAccessChain
            | Opcode::OpFunctionParameter
            | Opcode::OpFAdd
            | Opcode::OpFSub
            | Opcode::OpFMul
            | Opcode::OpFDiv
            | Opcode::OpFNegate
            | Opcode::OpIAdd
            | Opcode::OpISub
            | Opcode::OpIMul
            | Opcode::OpSDiv
            | Opcode::OpUDiv
            | Opcode::OpSMod
            | Opcode::OpUMod
            | Opcode::OpSNegate
            | Opcode::OpBitwiseAnd
            | Opcode::OpBitwiseOr
            | Opcode::OpBitwiseXor
            | Opcode::OpNot
            | Opcode::OpShiftLeftLogical
            | Opcode::OpShiftRightLogical
            | Opcode::OpShiftRightArithmetic
            | Opcode::OpFOrdEqual
            | Opcode::OpFOrdNotEqual
            | Opcode::OpFOrdLessThan
            | Opcode::OpFOrdGreaterThan
            | Opcode::OpFOrdLessThanEqual
            | Opcode::OpFOrdGreaterThanEqual
            | Opcode::OpIEqual
            | Opcode::OpINotEqual
            | Opcode::OpSLessThan
            | Opcode::OpSGreaterThan
            | Opcode::OpSLessThanEqual
            | Opcode::OpSGreaterThanEqual
            | Opcode::OpULessThan
            | Opcode::OpUGreaterThan
            | Opcode::OpULessThanEqual
            | Opcode::OpUGreaterThanEqual
            | Opcode::OpLogicalAnd
            | Opcode::OpLogicalOr
            | Opcode::OpLogicalNot
            | Opcode::OpLogicalEqual
            | Opcode::OpLogicalNotEqual
            | Opcode::OpSelect
            | Opcode::OpPhi
            | Opcode::OpConvertFToS
            | Opcode::OpConvertFToU
            | Opcode::OpConvertSToF
            | Opcode::OpConvertUToF
            | Opcode::OpBitcast
            | Opcode::OpCompositeConstruct
            | Opcode::OpCompositeExtract
            | Opcode::OpCompositeInsert
            | Opcode::OpVectorShuffle
            | Opcode::OpFunctionCall
            | Opcode::OpExtInst
            | Opcode::OpImageSampleImplicitLod
            | Opcode::OpImageSampleExplicitLod
            | Opcode::OpImageFetch
            | Opcode::OpImageRead
            | Opcode::OpDot => {
                // Both result type and result
                if pos < words.len() {
                    inst.result_type = Some(words[pos]);
                    pos += 1;
                }
                if pos < words.len() {
                    inst.result = Some(words[pos]);
                    pos += 1;
                }
            },
            Opcode::OpFunction => {
                // Result type and result
                if pos < words.len() {
                    inst.result_type = Some(words[pos]);
                    pos += 1;
                }
                if pos < words.len() {
                    inst.result = Some(words[pos]);
                    pos += 1;
                }
            },
            _ => {},
        }

        // Add remaining operands
        while pos < words.len() {
            inst.operands.push(Operand::Literal(words[pos]));
            pos += 1;
        }

        Ok(inst)
    }

    /// Decode all instructions
    pub fn decode_all(&mut self) -> SpirVResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        while !self.is_empty() {
            instructions.push(self.decode_instruction()?);
        }
        Ok(instructions)
    }

    /// Skip to position
    pub fn seek(&mut self, position: usize) {
        self.position = position.min(self.words.len());
    }
}

/// SPIR-V header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Version number
    pub version: u32,
    /// Generator magic number
    pub generator: u32,
    /// Bound (max ID + 1)
    pub bound: Id,
    /// Schema (reserved, must be 0)
    pub schema: u32,
}

impl Header {
    /// Create a new header
    pub fn new(version: u32, generator: u32, bound: Id) -> Self {
        Self {
            version,
            generator,
            bound,
            schema: 0,
        }
    }

    /// Get major version
    pub fn major_version(&self) -> u8 {
        ((self.version >> 16) & 0xFF) as u8
    }

    /// Get minor version
    pub fn minor_version(&self) -> u8 {
        ((self.version >> 8) & 0xFF) as u8
    }
}

/// Instruction stream for building SPIR-V
#[derive(Debug, Default)]
pub struct InstructionStream {
    /// Instructions
    instructions: Vec<Instruction>,
}

impl InstructionStream {
    /// Create a new instruction stream
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }

    /// Add an instruction
    pub fn push(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    /// Get instructions
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Take instructions
    pub fn take_instructions(self) -> Vec<Instruction> {
        self.instructions
    }

    /// Clear
    pub fn clear(&mut self) {
        self.instructions.clear();
    }

    /// Number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

/// Word buffer for efficient binary building
#[derive(Debug, Default)]
pub struct WordBuffer {
    /// Words
    words: Vec<u32>,
    /// Patch locations for forward references
    patches: Vec<PatchLocation>,
}

impl WordBuffer {
    /// Create a new word buffer
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            patches: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            words: Vec::with_capacity(capacity),
            patches: Vec::new(),
        }
    }

    /// Push a word
    pub fn push(&mut self, word: u32) {
        self.words.push(word);
    }

    /// Push a word with patch location
    pub fn push_patch(&mut self, word: u32, id: Id) {
        self.patches.push(PatchLocation {
            position: self.words.len(),
            target_id: id,
        });
        self.words.push(word);
    }

    /// Current position
    pub fn position(&self) -> usize {
        self.words.len()
    }

    /// Get words
    pub fn words(&self) -> &[u32] {
        &self.words
    }

    /// Take words
    pub fn take_words(self) -> Vec<u32> {
        self.words
    }

    /// Apply patches
    pub fn apply_patches<F>(&mut self, resolver: F)
    where
        F: Fn(Id) -> Option<u32>,
    {
        for patch in &self.patches {
            if let Some(value) = resolver(patch.target_id) {
                self.words[patch.position] = value;
            }
        }
    }

    /// Clear
    pub fn clear(&mut self) {
        self.words.clear();
        self.patches.clear();
    }
}

/// Patch location for forward references
#[derive(Debug, Clone)]
pub struct PatchLocation {
    /// Position in word buffer
    pub position: usize,
    /// Target ID to resolve
    pub target_id: Id,
}

/// Binary writer with sections
#[derive(Debug, Default)]
pub struct SectionedBinaryWriter {
    /// Capabilities section
    pub capabilities: InstructionStream,
    /// Extensions section
    pub extensions: InstructionStream,
    /// Ext inst imports section
    pub ext_inst_imports: InstructionStream,
    /// Memory model section
    pub memory_model: InstructionStream,
    /// Entry points section
    pub entry_points: InstructionStream,
    /// Execution modes section
    pub execution_modes: InstructionStream,
    /// Debug section (names, source)
    pub debug: InstructionStream,
    /// Annotations section (decorations)
    pub annotations: InstructionStream,
    /// Types and constants section
    pub types_constants: InstructionStream,
    /// Global variables section
    pub global_variables: InstructionStream,
    /// Functions section
    pub functions: InstructionStream,
}

impl SectionedBinaryWriter {
    /// Create a new sectioned writer
    pub fn new() -> Self {
        Self::default()
    }

    /// Encode all sections to binary
    pub fn encode(&self, version: u32, generator: u32, bound: Id) -> Vec<u32> {
        let mut encoder = BinaryEncoder::new();
        encoder.encode_header(version, generator, bound, 0);

        encoder.encode_instructions(self.capabilities.instructions());
        encoder.encode_instructions(self.extensions.instructions());
        encoder.encode_instructions(self.ext_inst_imports.instructions());
        encoder.encode_instructions(self.memory_model.instructions());
        encoder.encode_instructions(self.entry_points.instructions());
        encoder.encode_instructions(self.execution_modes.instructions());
        encoder.encode_instructions(self.debug.instructions());
        encoder.encode_instructions(self.annotations.instructions());
        encoder.encode_instructions(self.types_constants.instructions());
        encoder.encode_instructions(self.global_variables.instructions());
        encoder.encode_instructions(self.functions.instructions());

        encoder.take_words()
    }

    /// Clear all sections
    pub fn clear(&mut self) {
        self.capabilities.clear();
        self.extensions.clear();
        self.ext_inst_imports.clear();
        self.memory_model.clear();
        self.entry_points.clear();
        self.execution_modes.clear();
        self.debug.clear();
        self.annotations.clear();
        self.types_constants.clear();
        self.global_variables.clear();
        self.functions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_header() {
        let mut encoder = BinaryEncoder::new();
        encoder.encode_header(0x00010500, 0, 100, 0);

        let words = encoder.words();
        assert_eq!(words[0], SPIRV_MAGIC);
        assert_eq!(words[1], 0x00010500);
        assert_eq!(words[2], 0);
        assert_eq!(words[3], 100);
        assert_eq!(words[4], 0);
    }

    #[test]
    fn test_decode_header() {
        let words = [SPIRV_MAGIC, 0x00010500, 0, 100, 0];
        let mut decoder = BinaryDecoder::new(&words);

        let header = decoder.decode_header().unwrap();
        assert_eq!(header.version, 0x00010500);
        assert_eq!(header.major_version(), 1);
        assert_eq!(header.minor_version(), 5);
        assert_eq!(header.bound, 100);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut encoder = BinaryEncoder::new();
        encoder.encode_header(0x00010500, 0, 10, 0);

        // OpCapability Shader (1)
        let cap = Instruction::new(Opcode::OpCapability).with_literal(1);
        encoder.encode_instruction(&cap);

        // OpMemoryModel Logical GLSL450
        let mem = Instruction::new(Opcode::OpMemoryModel)
            .with_literal(0) // Logical
            .with_literal(1); // GLSL450
        encoder.encode_instruction(&mem);

        let words = encoder.words();
        let mut decoder = BinaryDecoder::new(words);

        // Decode header
        let header = decoder.decode_header().unwrap();
        assert_eq!(header.version, 0x00010500);

        // Decode instructions
        let inst1 = decoder.decode_instruction().unwrap();
        assert_eq!(inst1.opcode, Opcode::OpCapability);

        let inst2 = decoder.decode_instruction().unwrap();
        assert_eq!(inst2.opcode, Opcode::OpMemoryModel);
    }

    #[test]
    fn test_string_encoding() {
        let mut encoder = BinaryEncoder::new();
        encoder.encode_string("main");

        let words = encoder.words();
        // "main" = 4 chars + null = 5 bytes = 2 words
        assert_eq!(words.len(), 2);
    }
}
