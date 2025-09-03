// FIXME: Have `InstrumentContext` implement `Reencode` trait fully... rather than have it half way
// manually implemented and half-way reliant on wasm_encoder::reencode...

use crate::gas::InstrumentationKind;
use crate::AnalysisOutcome;
use std::convert::Infallible;
use wasm_encoder::reencode::{Error as ReencodeError, Reencode};
use wasm_encoder::{self as we};
use wasmparser as wp;

const PLACEHOLDER_FOR_NAMES: u8 = !0;

const GAS_GLOBAL: u32 = 0;
const STACK_GLOBAL: u32 = GAS_GLOBAL + 1;

/// By how many to adjust the references to globals in the instrumented module.
const G: u32 = STACK_GLOBAL + 1;

/// These function indices are known to be constant, as they are added at the beginning of the
/// imports section.
///
/// Doing so makes it much easier to transform references to other functions (basically add F to
/// all function indices)
const GAS_EXHAUSTED_FN: u32 = 0;
const STACK_EXHAUSTED_FN: u32 = GAS_EXHAUSTED_FN + 1;

const GAS_INSTRUMENTATION_FN: u32 = STACK_EXHAUSTED_FN + 1;

const MEMORY_COPY_INSTRUMENTATION_FN: u32 = GAS_INSTRUMENTATION_FN + 1;
const MEMORY_FILL_INSTRUMENTATION_FN: u32 = MEMORY_COPY_INSTRUMENTATION_FN + 1;
const MEMORY_INIT_INSTRUMENTATION_FN: u32 = MEMORY_FILL_INSTRUMENTATION_FN + 1;
const TABLE_COPY_INSTRUMENTATION_FN: u32 = MEMORY_INIT_INSTRUMENTATION_FN + 1;
const TABLE_FILL_INSTRUMENTATION_FN: u32 = TABLE_COPY_INSTRUMENTATION_FN + 1;
const TABLE_INIT_INSTRUMENTATION_FN: u32 = TABLE_FILL_INSTRUMENTATION_FN + 1;

/// See [`GAS_INSTRUMENTATION_FN`].
const RESERVE_STACK_INSTRUMENTATION_FN: u32 = TABLE_INIT_INSTRUMENTATION_FN + 1;

/// See [`RESERVE_STACK_INSTRUMENTATION_FN`].
const RELEASE_STACK_INSTRUMENTATION_FN: u32 = RESERVE_STACK_INSTRUMENTATION_FN + 1;

/// By how many to adjust the references to functions in the instrumented module.
const F: u32 = RELEASE_STACK_INSTRUMENTATION_FN + 1;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("could not reencode the element section")]
    ElementSection(#[source] ReencodeError<Infallible>),
    #[error("could not reencode a function type")]
    ReencodeFunctionType(#[source] ReencodeError<Infallible>),
    #[error("could not reencode the globals section")]
    ReencodeGlobals(#[source] ReencodeError<Infallible>),
    #[error("could not reencode the imports section")]
    ReencodeImports(#[source] ReencodeError<Infallible>),
    #[error("could not reencode a local type")]
    ReencodeLocal(#[source] ReencodeError<Infallible>),
    #[error("could not reencode the result types for a wrapper block")]
    BlockResults(#[source] ReencodeError<Infallible>),
    #[error("could not parse an element")]
    ParseElement(#[source] wp::BinaryReaderError),
    #[error("could not parse an element item")]
    ParseElementItem(#[source] wp::BinaryReaderError),
    #[error("could not parse an element expression")]
    ParseElementExpression(#[source] wp::BinaryReaderError),
    #[error("could not parse the function locals")]
    ParseLocals(#[source] wp::BinaryReaderError),
    #[error("could not parse a function local")]
    ParseLocal(#[source] wp::BinaryReaderError),
    #[error("could not parse the function operators")]
    ParseOperators(#[source] wp::BinaryReaderError),
    #[error("could not parse an operator")]
    ParseOperator(#[source] wp::BinaryReaderError),
    #[error("could not parse an export")]
    ParseExport(#[source] wp::BinaryReaderError),
    #[error("could not parse a global")]
    ParseGlobal(#[source] wp::BinaryReaderError),
    #[error("could not parse a name section entry")]
    ParseName(#[source] wp::BinaryReaderError),
    #[error("could not parse a name map entry")]
    ParseNameMapName(#[source] wp::BinaryReaderError),
    #[error("could not parse an indirect name map entry")]
    ParseIndirectNameMapName(#[source] wp::BinaryReaderError),
    #[error("could not parse a module section header")]
    ParseModuleSection(#[source] wp::BinaryReaderError),
    #[error("could not parse a type section entry")]
    ParseType(#[source] wp::BinaryReaderError),
    #[error("could not parse an import section entry")]
    ParseImport(#[source] wp::BinaryReaderError),
    #[error("could not parse a function section entry")]
    ParseFunctionTypeId(#[source] wp::BinaryReaderError),
    #[error("could not parse a constant expression operator")]
    ParseConstExprOperator(#[source] wp::BinaryReaderError),
    #[error("the analysis outcome missing a {0} entry for code section entry `{1}`")]
    FunctionMissingInAnalysisOutcome(&'static str, usize),
    #[error("module contains fewer function types than definitions")]
    InsufficientFunctionTypes,
    #[error("module contains a reference to an invalid type index")]
    InvalidTypeIndex,
    #[error("size for custom section {0} is out of input bounds")]
    CustomSectionRange(u8, usize),
    #[error("could not remap function index {0}")]
    RemapFunctionIndex(u32),
    #[error("could not remap global index {0}")]
    RemapGlobalIndex(u32),
    #[error("size for table section is out of input bounds")]
    TableSectionRange(usize),
    #[error("size for memory section is out of input bounds")]
    MemorySectionRange(usize),
    #[error("size for data count section is out of input bounds")]
    DataCountSection(usize),
    #[error("module contains too many imports")]
    TooManyImports,
}

pub(crate) struct InstrumentContext<'a> {
    analysis: &'a AnalysisOutcome,
    wasm: &'a [u8],
    import_env: &'a str,
    imported_functions: u32,
    op_cost: u32,
    max_stack_height: u32,

    type_section: we::TypeSection,
    import_section: we::ImportSection,
    function_section: we::FunctionSection,
    table_section: Option<we::RawSection<'a>>,
    memory_section: Option<we::RawSection<'a>>,
    global_section: we::GlobalSection,
    export_section: we::ExportSection,
    start_section: Option<we::StartSection>,
    element_section: we::ElementSection,
    datacount_section: Option<we::RawSection<'a>>,
    code_section: we::CodeSection,
    name_section: we::NameSection,
    raw_sections: Vec<we::RawSection<'a>>,

    types: Vec<we::FuncType>,
    function_types: std::vec::IntoIter<u32>,
}

struct InstrumentationReencoder {
    imported_functions: u32,
}

impl InstrumentationReencoder {
    fn namemap(&mut self, p: wp::NameMap, is_function: bool) -> Result<we::NameMap, Error> {
        let mut new_name_map = we::NameMap::new();
        for naming in p {
            let naming = naming.map_err(Error::ParseNameMapName)?;
            new_name_map.append(
                if is_function {
                    self.function_index(naming.index)
                } else {
                    naming.index
                },
                naming.name,
            );
        }
        Ok(new_name_map)
    }

    fn indirectnamemap(&mut self, p: wp::IndirectNameMap) -> Result<we::IndirectNameMap, Error> {
        let mut new_name_map = we::IndirectNameMap::new();
        for naming in p {
            let naming = naming.map_err(Error::ParseIndirectNameMapName)?;
            new_name_map.append(
                self.function_index(naming.index),
                &self.namemap(naming.names, false)?,
            );
        }
        Ok(new_name_map)
    }
}

impl<'a> Reencode for InstrumentationReencoder {
    type Error = Infallible; // FIXME

    fn global_index(&mut self, global: u32) -> u32 {
        global
            .checked_add(G)
            .ok_or(Error::RemapGlobalIndex(global))
            .expect("TODO: update wasm-encoder")
    }

    fn function_index(&mut self, func: u32) -> u32 {
        // FIXME: this breaks indexing for imported functions
        func.checked_add(F)
            .ok_or(Error::RemapFunctionIndex(func))
            .expect("TODO: update wasm-encoder")
            .checked_add(self.imported_functions)
            .expect("TODO: update wasm-encoder")
    }
}

trait InstructionSinkExt {
    /// ```wat
    /// i64.add128
    /// i64.popcnt
    /// i32.wrap_i64
    /// if
    ///     call $f
    ///     unreachable
    /// end
    /// ```
    fn checked_add(self, f: u32) -> Self;

    /// ```wat
    /// i64.const 0
    /// local.get $n
    /// i64.const 0
    /// i64.add128
    /// i64.popcnt
    /// i32.wrap_i64
    /// if
    ///     call $f
    ///     unreachable
    /// end
    /// ```
    fn checked_add_local_i64(self, n: u32, f: u32) -> Self;

    /// ```wat
    /// i64.sub128
    /// i64.popcnt
    /// i32.wrap_i64
    /// if
    ///     call $f
    ///     unreachable
    /// end
    /// ```
    fn checked_sub(self, f: u32) -> Self;

    /// ```wat
    /// i64.const 0
    /// local.get $n
    /// i64.const 0
    /// i64.sub128
    /// i64.popcnt
    /// i32.wrap_i64
    /// if
    ///     call $f
    ///     unreachable
    /// end
    /// ```
    fn checked_sub_local_i64(self, n: u32, f: u32) -> Self;

    /// ```wat
    /// i64.mul_wide_u
    /// i64.popcnt
    /// i32.wrap_i64
    /// if
    ///     call $f
    ///     unreachable
    /// end
    /// ```
    fn checked_mul(self, f: u32) -> Self;
}
impl InstructionSinkExt for &mut we::InstructionSink<'_> {
    fn checked_add(self, f: u32) -> Self {
        self.i64_add128()
            .i64_popcnt()
            .i32_wrap_i64()
            .if_(we::BlockType::Empty)
            .call(f)
            .unreachable()
            .end()
    }

    fn checked_add_local_i64(self, n: u32, f: u32) -> Self {
        self.i64_const(0).local_get(n).i64_const(0).checked_add(f)
    }

    fn checked_sub(self, f: u32) -> Self {
        self.i64_sub128()
            .i64_popcnt()
            .i32_wrap_i64()
            .if_(we::BlockType::Empty)
            .call(f)
            .unreachable()
            .end()
    }

    fn checked_sub_local_i64(self, n: u32, f: u32) -> Self {
        self.i64_const(0).local_get(n).i64_const(0).checked_sub(f)
    }

    fn checked_mul(self, f: u32) -> Self {
        self.i64_mul_wide_u()
            .i64_popcnt()
            .i32_wrap_i64()
            .if_(we::BlockType::Empty)
            .call(f)
            .unreachable()
            .end()
    }
}

impl<'a> InstrumentContext<'a> {
    pub(crate) fn new(
        wasm: &'a [u8],
        import_env: &'a str,
        analysis: &'a AnalysisOutcome,
        op_cost: u32,
        max_stack_height: u32,
    ) -> Self {
        Self {
            analysis,
            wasm,
            import_env,
            imported_functions: 0,
            op_cost,
            max_stack_height,

            type_section: we::TypeSection::new(),
            import_section: we::ImportSection::new(),
            function_section: we::FunctionSection::new(),
            table_section: None,
            memory_section: None,
            global_section: we::GlobalSection::new(),
            export_section: we::ExportSection::new(),
            start_section: None,
            element_section: we::ElementSection::new(),
            datacount_section: None,
            code_section: we::CodeSection::new(),
            name_section: we::NameSection::new(),
            raw_sections: vec![],

            types: vec![],
            function_types: vec![].into_iter(),
        }
    }

    fn schedule_section(&mut self, id: u8) {
        self.raw_sections.push(we::RawSection { id, data: &[] });
    }

    pub(crate) fn run(mut self) -> Result<Vec<u8>, Error> {
        let parser = wp::Parser::new(0);
        let mut renc = InstrumentationReencoder {
            imported_functions: 0,
        };
        for payload in parser.parse_all(self.wasm) {
            let payload = payload.map_err(Error::ParseModuleSection)?;
            match payload {
                // These two payload types are (re-)generated by wasm_encoder.
                wp::Payload::Version { .. } => {}
                wp::Payload::End(_) => {}
                // We must manually reconstruct the type section because we’re appending types to
                // it.
                wp::Payload::TypeSection(types) => {
                    for ty in types.into_iter_err_on_gc_types() {
                        let ty = ty.map_err(Error::ParseType)?;
                        let ty = renc.func_type(ty).map_err(Error::ReencodeFunctionType)?;
                        self.type_section.ty().func_type(&ty);
                        self.types.push(ty);
                    }
                }

                // We must manually reconstruct the imports section because we’re appending imports
                // to it.
                wp::Payload::ImportSection(imports) => {
                    self.maybe_add_imports();
                    for import in imports {
                        let import = import.map_err(Error::ParseImport)?;
                        if let wp::TypeRef::Func(..) = import.ty {
                            self.imported_functions = self
                                .imported_functions
                                .checked_add(1)
                                .ok_or(Error::TooManyImports)?;
                        }
                        renc.parse_import(&mut self.import_section, import)
                            .map_err(Error::ReencodeImports)?;
                    }
                    if self.imported_functions.checked_add(F).is_none() {
                        return Err(Error::TooManyImports);
                    }
                    renc.imported_functions = self.imported_functions;
                }
                wp::Payload::StartSection { func, .. } => {
                    self.start_section = Some(we::StartSection {
                        function_index: renc.start_section(func),
                    });
                }
                wp::Payload::ElementSection(reader) => {
                    renc.parse_element_section(&mut self.element_section, reader)
                        .map_err(Error::ElementSection)?;
                }
                wp::Payload::FunctionSection(reader) => {
                    self.maybe_add_functions();
                    // We need to remember function type indices
                    let fn_types = reader
                        .into_iter()
                        .collect::<Result<Vec<u32>, _>>()
                        .map_err(Error::ParseFunctionTypeId)?;
                    for fnty in &fn_types {
                        self.function_section.function(*fnty);
                    }
                    self.function_types = fn_types.into_iter();
                }
                wp::Payload::TableSection(..) => {
                    let (id, range) = payload.as_section().unwrap();
                    let len = range.len();
                    self.table_section = Some(we::RawSection {
                        id,
                        data: self.wasm.get(range).ok_or(Error::TableSectionRange(len))?,
                    });
                }
                wp::Payload::MemorySection(..) => {
                    let (id, range) = payload.as_section().unwrap();
                    let len = range.len();
                    self.memory_section = Some(we::RawSection {
                        id,
                        data: self.wasm.get(range).ok_or(Error::MemorySectionRange(len))?,
                    });
                }
                wp::Payload::CodeSectionStart { .. } => {
                    self.maybe_add_code();
                }
                wp::Payload::CodeSectionEntry(reader) => {
                    let type_index = self
                        .function_types
                        .next()
                        .ok_or(Error::InsufficientFunctionTypes)?;
                    self.transform_code_section(&mut renc, reader, type_index)?;
                }
                wp::Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export.map_err(Error::ParseExport)?;
                        let (kind, index) = match export.kind {
                            wp::ExternalKind::Func => {
                                (we::ExportKind::Func, renc.function_index(export.index))
                            }
                            wp::ExternalKind::Table => (we::ExportKind::Table, export.index),
                            wp::ExternalKind::Memory => (we::ExportKind::Memory, export.index),
                            wp::ExternalKind::Global => (we::ExportKind::Global, export.index),
                            wp::ExternalKind::Tag => (we::ExportKind::Tag, export.index),
                        };
                        self.export_section.export(export.name, kind, index);
                    }
                }
                wp::Payload::GlobalSection(reader) => {
                    self.maybe_add_globals();
                    for global in reader {
                        let global = global.map_err(Error::ParseGlobal)?;
                        renc.parse_global(&mut self.global_section, global)
                            .map_err(Error::ReencodeGlobals)?;
                    }
                }
                wp::Payload::DataCountSection { .. } => {
                    let (id, range) = payload.as_section().unwrap();
                    let len = range.len();
                    self.datacount_section = Some(we::RawSection {
                        id,
                        data: self.wasm.get(range).ok_or(Error::DataCountSection(len))?,
                    });
                }
                wp::Payload::CustomSection(reader) if reader.name() == "name" => {
                    let wasmparser::KnownCustom::Name(names) = reader.as_known() else {
                        continue;
                    };
                    if let Ok(_) = self.transform_name_section(&mut renc, names) {
                        // Keep valid name sections only. These sections don't have
                        // semantic purposes, so it isn't a big deal if we only keep the
                        // old section, or don't transform at all.
                        //
                        // (This is largely useful for fuzzing only)
                        self.schedule_section(PLACEHOLDER_FOR_NAMES)
                    }
                }
                // All the other sections are transparently copied over (they cannot reference a
                // function id, global id, or we don’t know how to handle it anyhow)
                _ => {
                    let (id, range) = payload
                        .as_section()
                        .expect("any non-section payloads should have been handled already");
                    let len = range.len();
                    self.raw_sections.push(wasm_encoder::RawSection {
                        id,
                        data: self
                            .wasm
                            .get(range)
                            .ok_or(Error::CustomSectionRange(id, len))?,
                    });
                }
            }
        }

        // The type and import sections always come first in a module. They may potentially be
        // preceded or interspersed by custom sections in the original module, so we’re just hoping
        // that the ordering doesn’t matter for tests…
        let mut output = wasm_encoder::Module::new();
        if !self.type_section.is_empty() {
            output.section(&self.type_section);
        }
        if !self.import_section.is_empty() {
            output.section(&self.import_section);
        }
        if !self.function_section.is_empty() {
            output.section(&self.function_section);
        }
        if let Some(section) = self.table_section {
            output.section(&section);
        }
        if let Some(section) = self.memory_section {
            output.section(&section);
        }
        if !self.global_section.is_empty() {
            output.section(&self.global_section);
        }
        if !self.export_section.is_empty() {
            output.section(&self.export_section);
        }
        if let Some(section) = self.start_section {
            output.section(&section);
        }
        if !self.element_section.is_empty() {
            output.section(&self.element_section);
        }
        if let Some(section) = self.datacount_section {
            output.section(&section);
        }
        if !self.code_section.is_empty() {
            output.section(&self.code_section);
        }
        for section in self.raw_sections {
            match section.id {
                PLACEHOLDER_FOR_NAMES => output.section(&self.name_section),
                _ => output.section(&section),
            };
        }
        // TODO: remove once fully implemented
        std::fs::write("/tmp/out.wasm", output.as_slice()).unwrap();
        Ok(output.finish())
    }

    fn transform_code_section(
        &mut self,
        renc: &mut InstrumentationReencoder,
        reader: wp::FunctionBody,
        func_type_idx: u32,
    ) -> Result<(), Error> {
        let func_type_idx_usize =
            usize::try_from(func_type_idx).map_err(|_| Error::InvalidTypeIndex)?;
        let func_type = self
            .types
            .get(func_type_idx_usize)
            .ok_or(Error::InvalidTypeIndex)?;
        let locals = reader
            .get_locals_reader()
            .map_err(Error::ParseLocals)?
            .into_iter()
            .map(|v| {
                v.map_err(Error::ParseLocal)
                    .and_then(|(c, t)| Ok((c, renc.val_type(t).map_err(Error::ReencodeLocal)?)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut new_function = we::Function::new(locals);
        let code_idx = self
            .code_section
            .len()
            .saturating_sub(F - GAS_INSTRUMENTATION_FN) as usize;
        macro_rules! get_idx {
            (analysis . $field: ident) => {{
                let f = self.analysis.$field.get(code_idx);
                const NAME: &str = stringify!($field);
                f.ok_or(Error::FunctionMissingInAnalysisOutcome(NAME, code_idx))
            }};
        }
        let gas_costs = get_idx!(analysis.gas_costs)?;
        let gas_kinds = get_idx!(analysis.gas_kinds)?;
        let gas_offsets = get_idx!(analysis.gas_offsets)?;
        let stack_sz = *get_idx!(analysis.function_operand_stack_sizes)?;
        let frame_sz = *get_idx!(analysis.function_frame_sizes)?;

        let mut instrumentation_points = gas_offsets
            .iter()
            .zip(gas_costs.iter())
            .zip(gas_kinds.iter())
            .peekable();
        let mut operators = reader
            .get_operators_reader()
            .map_err(Error::ParseOperators)?;

        // In order to enable us to insert the code to release the stack allocation, we’ll wrap the
        // function body into a `block` and insert the instrumentation after the block ends… This
        // additional wrapping block allows us to “intercept” various branching instructions with
        // frame depths that would otherwise lead to a return. This is especially important when
        // these branching instructions are conditional: we could replace `br $well_chosen_index`
        // with a `return` and handle it much the same way, but we can’t do anything of the sort
        // for `br_if $well_chosen_index`.
        let (params, results) = (func_type.params(), func_type.results());
        // NOTE: Function parameters become locals, rather than operands, so we don’t need to
        // handle them in any way when inserting the block.
        let block_type = match (params, results) {
            (_, []) => we::BlockType::Empty,
            (_, [result]) => we::BlockType::Result(*result),
            ([], _) => we::BlockType::FunctionType(func_type_idx),
            (_, results) => {
                let new_block_type_idx = self.type_section.len();
                self.type_section
                    .ty()
                    .function(std::iter::empty(), results.iter().copied());
                we::BlockType::FunctionType(new_block_type_idx)
            }
        };

        let should_instrument_stack = stack_sz != 0 || frame_sz != 0;
        if should_instrument_stack {
            new_function.instruction(&we::Instruction::Block(block_type));
            new_function.instruction(&we::Instruction::I64Const(stack_sz as i64));
            new_function.instruction(&we::Instruction::I64Const(frame_sz as i64));
            new_function.instruction(&we::Instruction::Call(RESERVE_STACK_INSTRUMENTATION_FN));
        }

        while !operators.eof() {
            let (op, offset) = operators.read_with_offset().map_err(Error::ParseOperator)?;
            let end_offset = operators.original_position();
            while instrumentation_points.peek().map(|((o, _), _)| **o) == Some(offset) {
                let ((_, g), k) = instrumentation_points.next().expect("we just peeked");
                if !matches!(k, InstrumentationKind::Unreachable) {
                    call_gas_instrumentation(&mut new_function, k, *g, self.imported_functions)
                }
            }
            match op {
                wp::Operator::RefFunc { function_index } => new_function.instruction(
                    &we::Instruction::RefFunc(renc.function_index(function_index)),
                ),
                wp::Operator::Call { function_index } => new_function
                    .instruction(&we::Instruction::Call(renc.function_index(function_index))),
                wp::Operator::ReturnCall { function_index } => {
                    call_unstack_instrumentation(&mut new_function, stack_sz, frame_sz);
                    new_function.instruction(&we::Instruction::ReturnCall(
                        renc.function_index(function_index),
                    ))
                }
                wp::Operator::ReturnCallIndirect { .. } => {
                    call_unstack_instrumentation(&mut new_function, stack_sz, frame_sz);
                    new_function.raw(self.wasm[offset..end_offset].iter().copied())
                }
                wp::Operator::Return => {
                    // FIXME: we could replace these `return`s with `br $well_chosen_index`
                    // targetting the block we inserted around the function body.
                    call_unstack_instrumentation(&mut new_function, stack_sz, frame_sz);
                    new_function.instruction(&we::Instruction::Return)
                }
                wp::Operator::End if operators.eof() => {
                    // This is the last function end…
                    if should_instrument_stack {
                        new_function.instruction(&we::Instruction::End);
                        call_unstack_instrumentation(&mut new_function, stack_sz, frame_sz);
                    }
                    new_function.instruction(&we::Instruction::End)
                }
                _ => new_function.raw(self.wasm[offset..end_offset].iter().copied()),
            };
        }

        self.code_section.function(&new_function);
        Ok(())
    }

    fn maybe_add_imports(&mut self) {
        if self.import_section.is_empty() {
            // By adding the type at the end of the type section we guarantee that any other
            // type references remain valid.
            let exhausted_fnty = self.type_section.len();
            self.type_section.ty().function([], []);
            // By inserting the imports at the beginning of the import section we make the new
            // function index mapping trivial (it is always just an increment by F)
            debug_assert_eq!(self.import_section.len(), GAS_EXHAUSTED_FN);
            self.import_section.import(
                self.import_env,
                "finite_wasm_gas_exhausted",
                we::EntityType::Function(exhausted_fnty),
            );
            debug_assert_eq!(self.import_section.len(), STACK_EXHAUSTED_FN);
            self.import_section.import(
                self.import_env,
                "finite_wasm_stack_exhausted",
                we::EntityType::Function(exhausted_fnty),
            );
            debug_assert_eq!(self.import_section.len(), 2);
        }
    }

    fn maybe_add_functions(&mut self) {
        self.maybe_add_imports();
        if self.function_section.is_empty() {
            // By adding the type at the end of the type section we guarantee that any other
            // type references remain valid.
            let gas_fnty = self.type_section.len();
            self.type_section.ty().function([we::ValType::I64], []);
            let stack_fnty = self.type_section.len();
            self.type_section
                .ty()
                .function([we::ValType::I64, we::ValType::I64], []);
            // FIXME: these operators actually take two additional arguments i.e. for memory [i32
            // i32 i32] and for table [i32 ref i32]. It might be interesting for the
            // instrumentation to modify fees based on the value being filled, but it would require
            // a separate instrumentation for every single *ref type for tables as well as some
            // knowledge in instrumentation as to what sort of references `table.fill` is operating
            // on. For now we punt on this by only ever taking the last argument which for all of
            // the bulk memory operations represents the number of elements to work on (i.e. the
            // scale of the work.) This also makes these intrinsics compatible with non-multi-value
            // VMs still.
            let copy_init_fill_fnty = self.type_section.len();
            self.type_section.ty().function(
                [we::ValType::I32, we::ValType::I64, we::ValType::I64],
                [we::ValType::I32],
            );

            // By inserting the functions at the beginning of the function section we make the new
            // function index mapping trivial (it is always just an increment by F)
            debug_assert_eq!(
                self.function_section.len(),
                GAS_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(gas_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                MEMORY_COPY_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                MEMORY_FILL_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                MEMORY_INIT_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                TABLE_COPY_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                TABLE_FILL_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                TABLE_INIT_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(copy_init_fill_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                RESERVE_STACK_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(stack_fnty);
            debug_assert_eq!(
                self.function_section.len(),
                RELEASE_STACK_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.function_section.function(stack_fnty);

            debug_assert_eq!(self.function_section.len(), F - GAS_INSTRUMENTATION_FN);
        }
    }

    fn maybe_add_globals(&mut self) {
        if self.global_section.is_empty() {
            debug_assert_eq!(self.global_section.len(), GAS_GLOBAL);
            self.global_section.global(
                we::GlobalType {
                    val_type: we::ValType::I64,
                    mutable: true,
                    shared: false,
                },
                &we::ConstExpr::i64_const(0),
            );
            debug_assert_eq!(self.global_section.len(), STACK_GLOBAL);
            self.global_section.global(
                we::GlobalType {
                    val_type: we::ValType::I64,
                    mutable: true,
                    shared: false,
                },
                &we::ConstExpr::i64_const(self.max_stack_height.into()),
            );
            debug_assert_eq!(self.global_section.len(), G);

            self.export_section.export(
                "\0finite_wasm_remaining_gas",
                we::ExportKind::Global,
                GAS_GLOBAL,
            );
        }
    }

    fn maybe_add_code(&mut self) {
        self.maybe_add_functions();
        self.maybe_add_globals();
        if self.code_section.is_empty() {
            // (param $n i64)
            let mut finite_wasm_gas = we::Function::new([]);
            finite_wasm_gas
                .instructions()
                .global_get(GAS_GLOBAL)
                // $gas
                .checked_sub_local_i64(0, GAS_EXHAUSTED_FN)
                // $gas - $n
                .global_set(GAS_GLOBAL)
                .end();
            debug_assert_eq!(
                self.code_section.len(),
                GAS_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&finite_wasm_gas);

            // (param $count i32) (param $linear i64) (param $constant i64) (result i32)
            let mut linear_gas = we::Function::new([]);
            linear_gas
                .instructions()
                .global_get(GAS_GLOBAL)
                // $gas
                .i64_const(0)
                // $gas | 0
                .local_get(0)
                .i64_extend_i32_u()
                // $gas | 0 | $count
                .local_get(1)
                // $gas | 0 | $count | $linear
                .checked_mul(GAS_EXHAUSTED_FN)
                // $gas | 0 | $count * $linear
                .checked_add_local_i64(2, GAS_EXHAUSTED_FN)
                // $gas | 0 | $count * $linear + $constant
                .i64_const(0)
                // $gas | 0 | $count * $linear + $constant | 0
                .checked_sub(GAS_EXHAUSTED_FN)
                // $gas - $count * $linear + $constant
                .global_set(GAS_GLOBAL)
                .local_get(0)
                .end();
            debug_assert_eq!(
                self.code_section.len(),
                MEMORY_COPY_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);
            debug_assert_eq!(
                self.code_section.len(),
                MEMORY_FILL_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);
            debug_assert_eq!(
                self.code_section.len(),
                MEMORY_INIT_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);
            debug_assert_eq!(
                self.code_section.len(),
                TABLE_COPY_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);
            debug_assert_eq!(
                self.code_section.len(),
                TABLE_FILL_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);
            debug_assert_eq!(
                self.code_section.len(),
                TABLE_INIT_INSTRUMENTATION_FN - GAS_INSTRUMENTATION_FN
            );
            self.code_section.function(&linear_gas);

            // (param $operand_size i64) (param $frame_size i64)
            let mut finite_wasm_stack = we::Function::new([]);
            finite_wasm_stack
                .instructions()
                .global_get(STACK_GLOBAL)
                // $stack
                .checked_sub_local_i64(0, STACK_EXHAUSTED_FN)
                // $stack - $operand_size
                .checked_sub_local_i64(1, STACK_EXHAUSTED_FN)
                // $stack - $operand_size - $frame_size
                .global_set(STACK_GLOBAL)
                .global_get(GAS_GLOBAL)
                .i64_const(0)
                .local_get(1)
                // $gas | 0 | $frame_size
                .i64_const(8)
                // $gas | 0 | $frame_size | 8
                .i64_div_u()
                // $gas | 0 | $frame_size / 8
                .i64_const(self.op_cost.into())
                // $gas | 0 | $frame_size / 8 | $op_cost
                .checked_mul(GAS_EXHAUSTED_FN)
                // $gas | 0 | $frame_size / 8 * $op_cost
                .i64_const(0)
                // $gas | 0 | $frame_size / 8 * $op_cost | 0
                .checked_sub(GAS_EXHAUSTED_FN)
                // $gas - $frame_size / 8 * $op_cost
                .global_set(GAS_GLOBAL)
                .local_get(1)
                // $frame_size
                .i64_const(8)
                // $frame_size | 8
                .i64_rem_u()
                .i32_wrap_i64()
                // $frame_size % 8
                .if_(we::BlockType::Empty)
                .global_get(GAS_GLOBAL)
                // $gas
                .checked_sub_local_i64(1, GAS_EXHAUSTED_FN)
                // $gas - $frame_size
                .global_set(GAS_GLOBAL)
                .end()
                .end();
            self.code_section.function(&finite_wasm_stack);

            // (param $operand_size i64) (param $frame_size i64)
            let mut finite_wasm_unstack = we::Function::new([]);
            finite_wasm_unstack
                .instructions()
                .global_get(STACK_GLOBAL)
                .checked_add_local_i64(0, STACK_EXHAUSTED_FN)
                // $stack + $operand_size
                .checked_add_local_i64(1, STACK_EXHAUSTED_FN)
                // $stack + $operand_size + $frame_size
                .global_set(STACK_GLOBAL)
                .end();
            self.code_section.function(&finite_wasm_unstack);

            debug_assert_eq!(self.code_section.len(), F - GAS_INSTRUMENTATION_FN);
        }
    }

    fn transform_name_section(
        &mut self,
        renc: &mut InstrumentationReencoder,
        names: wp::NameSectionReader,
    ) -> Result<(), Error> {
        for name in names {
            let name = name.map_err(Error::ParseName)?;
            match name {
                wp::Name::Module { name, .. } => self.name_section.module(name),
                wp::Name::Function(map) => {
                    let mut new_name_map = we::NameMap::new();
                    new_name_map.append(GAS_EXHAUSTED_FN, "finite_wasm_gas_exhausted");
                    new_name_map.append(STACK_EXHAUSTED_FN, "finite_wasm_stack_exhausted");
                    new_name_map.append(
                        self.imported_functions + GAS_INSTRUMENTATION_FN,
                        "finite_wasm_gas",
                    );
                    new_name_map.append(
                        self.imported_functions + RESERVE_STACK_INSTRUMENTATION_FN,
                        "finite_wasm_stack",
                    );
                    new_name_map.append(
                        self.imported_functions + RELEASE_STACK_INSTRUMENTATION_FN,
                        "finite_wasm_unstack",
                    );
                    for naming in map {
                        let naming = naming.map_err(Error::ParseNameMapName)?;
                        new_name_map.append(renc.function_index(naming.index), naming.name);
                    }
                    self.name_section.functions(&new_name_map)
                }
                wp::Name::Local(map) => self.name_section.locals(&renc.indirectnamemap(map)?),
                wp::Name::Label(map) => self.name_section.labels(&renc.indirectnamemap(map)?),
                wp::Name::Type(map) => self.name_section.types(&renc.namemap(map, false)?),
                wp::Name::Table(map) => self.name_section.tables(&renc.namemap(map, false)?),
                wp::Name::Memory(map) => self.name_section.memories(&renc.namemap(map, false)?),
                wp::Name::Global(map) => self.name_section.globals(&renc.namemap(map, false)?),
                wp::Name::Element(map) => self.name_section.elements(&renc.namemap(map, false)?),
                wp::Name::Data(map) => self.name_section.data(&renc.namemap(map, false)?),
                wp::Name::Field(map) => self.name_section.fields(&renc.indirectnamemap(map)?),
                wp::Name::Tag(map) => self.name_section.tag(&renc.namemap(map, false)?),
                wp::Name::Unknown { .. } => {}
            }
        }
        Ok(())
    }
}

fn call_unstack_instrumentation(
    func: &mut we::Function,
    max_operand_stack_size: u64,
    function_frame_size: u64,
) {
    if max_operand_stack_size != 0 || function_frame_size != 0 {
        // These casts being able to wrap-around is intentional. The callee must reinterpret these
        // back to unsigned.
        func.instruction(&we::Instruction::I64Const(max_operand_stack_size as i64));
        func.instruction(&we::Instruction::I64Const(function_frame_size as i64));
        func.instruction(&we::Instruction::Call(RELEASE_STACK_INSTRUMENTATION_FN));
    }
}

fn call_gas_instrumentation(
    func: &mut we::Function,
    k: &InstrumentationKind,
    gas: crate::Fee,
    imported_functions: u32,
) {
    // NOTE: We have already verified that `imported_functions + F`  fits in u32
    if matches!(gas, crate::Fee::ZERO) {
        return;
    } else if gas.linear == 0 {
        // The reinterpreting cast is intentional here. On the other side the host function is
        // expected to reinterpret the argument back to u64.
        func.instruction(&we::Instruction::I64Const(gas.constant as i64));
        func.instruction(&we::Instruction::Call(
            imported_functions + GAS_INSTRUMENTATION_FN,
        ));
    } else {
        match k {
            InstrumentationKind::TableInit => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions + TABLE_INIT_INSTRUMENTATION_FN,
                ));
            }
            InstrumentationKind::TableFill => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions + TABLE_FILL_INSTRUMENTATION_FN,
                ));
            }
            InstrumentationKind::TableCopy => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions + TABLE_COPY_INSTRUMENTATION_FN,
                ));
            }
            InstrumentationKind::MemoryInit => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions + MEMORY_INIT_INSTRUMENTATION_FN,
                ));
            }
            InstrumentationKind::MemoryFill => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions + MEMORY_FILL_INSTRUMENTATION_FN,
                ));
            }
            InstrumentationKind::MemoryCopy => {
                func.instruction(&we::Instruction::I64Const(gas.linear as i64));
                func.instruction(&we::Instruction::I64Const(gas.constant as i64));
                func.instruction(&we::Instruction::Call(
                    imported_functions
                        + MEMORY_COPY_INSTRUMENTATION_FN.saturating_add(imported_functions),
                ));
            }
            _ => {
                panic!("configuration error, linear gas fees are only applicable to aggregate operations");
            }
        }
    }
}
