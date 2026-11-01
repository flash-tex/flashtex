//! Explicit immutable borrowed VF/TFM/font graph; no file or font discovery.
use crate::{
    encoding::{BoundTfmFont, GlyphIdentity},
    invalid,
    tfm::{FixWord, Tfm},
    vf::{Command, Register, VirtualFont},
    Coordinate, Result,
};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKey {
    Physical {
        font_sha256: String,
        tfm_sha256: String,
        face_index: u32,
    },
    CffPhysical {
        font_sha256: String,
        cff_sha256: String,
        tfm_sha256: String,
        encoding_sha256: String,
        face_index: u32,
    },
    Virtual {
        vf_sha256: String,
        tfm_sha256: String,
    },
}
fn valid_key(key: &ResourceKey) -> bool {
    match key {
        ResourceKey::Physical {
            font_sha256,
            tfm_sha256,
            face_index,
        } => crate::valid_hash(font_sha256) && crate::valid_hash(tfm_sha256) && *face_index == 0,
        ResourceKey::CffPhysical {
            font_sha256,
            cff_sha256,
            tfm_sha256,
            encoding_sha256,
            face_index,
        } => {
            crate::valid_hash(font_sha256)
                && crate::valid_hash(cff_sha256)
                && crate::valid_hash(tfm_sha256)
                && crate::valid_hash(encoding_sha256)
                && *face_index == 0
        }
        ResourceKey::Virtual {
            vf_sha256,
            tfm_sha256,
        } => crate::valid_hash(vf_sha256) && crate::valid_hash(tfm_sha256),
    }
}
pub enum Resource<'a> {
    Physical(&'a BoundTfmFont<'a>),
    CffPhysical(&'a crate::cff::BoundCffTfmFont<'a>),
    Virtual {
        vf: &'a VirtualFont,
        tfm: &'a Tfm,
        fonts: BTreeMap<i32, ResourceKey>,
    },
}
impl Resource<'_> {
    pub fn key(&self) -> ResourceKey {
        match self {
            Self::Physical(binding) => ResourceKey::Physical {
                font_sha256: binding.font().descriptor().sha256.clone(),
                tfm_sha256: binding.tfm().source_sha256.clone(),
                face_index: binding.font().descriptor().face_index,
            },
            Self::CffPhysical(binding) => ResourceKey::CffPhysical {
                font_sha256: binding.identity().font_sha256.clone(),
                cff_sha256: binding.identity().cff_sha256.clone(),
                tfm_sha256: binding.tfm().source_sha256.clone(),
                encoding_sha256: binding.encoding().encoding_sha256().into(),
                face_index: binding.identity().face_index,
            },
            Self::Virtual { vf, tfm, .. } => ResourceKey::Virtual {
                vf_sha256: vf.source_sha256.clone(),
                tfm_sha256: tfm.source_sha256.clone(),
            },
        }
    }
    fn tfm(&self) -> &Tfm {
        match self {
            Self::Physical(b) => b.tfm(),
            Self::CffPhysical(b) => b.tfm(),
            Self::Virtual { tfm, .. } => tfm,
        }
    }
}
pub struct ResourceGraph<'a> {
    resources: BTreeMap<ResourceKey, Resource<'a>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceStep {
    pub resource: ResourceKey,
    pub character: u8,
    pub command_index: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedPlacement {
    Glyph {
        resource: ResourceKey,
        glyph_id: u16,
        tfm_code: u8,
        x: Coordinate,
        y: Coordinate,
        scale: Coordinate,
        source: Vec<SourceStep>,
    },
    Rule {
        x: Coordinate,
        y: Coordinate,
        width: Coordinate,
        height: Coordinate,
        source: Vec<SourceStep>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedPacket {
    pub resource: ResourceKey,
    pub character: u8,
    pub width: FixWord,
    pub placements: Vec<NestedPlacement>,
}
struct Budget {
    nodes: usize,
    commands: usize,
    outputs: usize,
}
impl<'a> Default for ResourceGraph<'a> {
    fn default() -> Self {
        Self::new()
    }
}
impl<'a> ResourceGraph<'a> {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
        }
    }
    pub fn insert(&mut self, resource: Resource<'a>) -> Result<ResourceKey> {
        if self.resources.len() >= 4096 {
            return Err(invalid("VF resource graph budget"));
        }
        let key = resource.key();
        if !valid_key(&key) {
            return Err(invalid("invalid VF resource hash key"));
        }
        if let Resource::Virtual { vf, fonts, .. } = &resource {
            if fonts.len() != vf.fonts().len()
                || fonts
                    .iter()
                    .any(|(id, key)| !vf.fonts().contains_key(id) || !valid_key(key))
            {
                return Err(invalid("VF declared local resource key mismatch"));
            }
        }
        if self.resources.contains_key(&key) {
            return Err(invalid("duplicate VF resource identity"));
        }
        self.resources.insert(key.clone(), resource);
        Ok(key)
    }
    pub fn expand(&self, key: &ResourceKey, code: u8) -> Result<NestedPacket> {
        self.expand_inner(
            key,
            code,
            &mut Vec::new(),
            &mut Budget {
                nodes: 0,
                commands: 0,
                outputs: 0,
            },
        )
    }
    fn expand_inner(
        &self,
        key: &ResourceKey,
        code: u8,
        stack: &mut Vec<(ResourceKey, u8)>,
        budget: &mut Budget,
    ) -> Result<NestedPacket> {
        if stack.len() > 32 || stack.contains(&(key.clone(), code)) {
            return Err(invalid("nested VF cycle/depth budget"));
        }
        budget.nodes += 1;
        if budget.nodes > 4096 {
            return Err(invalid("nested VF node budget"));
        }
        let resource = self
            .resources
            .get(key)
            .ok_or_else(|| invalid("nested VF resource hash binding missing"))?;
        let tfm = resource.tfm();
        let width = tfm
            .char_metrics(code)
            .ok_or_else(|| invalid("nested VF TFM character missing"))?
            .width;
        let physical_identity = match resource {
            Resource::Physical(binding) => Some(binding.map_code(code)?.0),
            Resource::CffPhysical(binding) => Some(binding.map_code(code)?.0),
            _ => None,
        };
        if let Some(identity) = physical_identity {
            let GlyphIdentity::Original(glyph_id) = identity else {
                return Err(invalid("nested VF explicit .notdef"));
            };
            budget.outputs += 1;
            if budget.outputs > 100000 {
                return Err(invalid("nested VF output budget"));
            }
            return Ok(NestedPacket {
                resource: key.clone(),
                character: code,
                width,
                placements: vec![NestedPlacement::Glyph {
                    resource: key.clone(),
                    glyph_id,
                    tfm_code: code,
                    x: Coordinate::from_integer(0),
                    y: Coordinate::from_integer(0),
                    scale: Coordinate::from_integer(1),
                    source: vec![SourceStep {
                        resource: key.clone(),
                        character: code,
                        command_index: None,
                    }],
                }],
            });
        }
        let Resource::Virtual { vf, fonts, .. } = resource else {
            unreachable!()
        };
        if vf.design_size != tfm.design_size
            || (vf.checksum != 0 && tfm.checksum != 0 && vf.checksum != tfm.checksum)
        {
            return Err(invalid("nested VF TFM header mismatch"));
        }
        let packet = vf
            .packet(code as u32)
            .ok_or_else(|| invalid("nested VF packet missing"))?;
        if packet.width != width {
            return Err(invalid("nested VF packet width mismatch"));
        }
        for (id, definition) in vf.fonts() {
            let child = fonts
                .get(id)
                .and_then(|k| self.resources.get(k))
                .ok_or_else(|| invalid("nested VF local resource missing"))?;
            if child.tfm().design_size != definition.design_size
                || (definition.checksum != 0
                    && child.tfm().checksum != 0
                    && definition.checksum != child.tfm().checksum)
            {
                return Err(invalid("nested VF local metric identity mismatch"));
            }
        }
        stack.push((key.clone(), code));
        #[derive(Clone, Copy)]
        struct State {
            h: Coordinate,
            v: Coordinate,
            registers: [i32; 4],
        }
        let mut state = State {
            h: Coordinate::from_integer(0),
            v: Coordinate::from_integer(0),
            registers: [0; 4],
        };
        let mut saved = Vec::new();
        let mut font = vf.first_font();
        let mut placements = Vec::new();
        for (command_index, command) in packet.commands.iter().enumerate() {
            budget.commands += 1;
            if budget.commands > 1_000_000 {
                return Err(invalid("nested VF command budget"));
            }
            let step = SourceStep {
                resource: key.clone(),
                character: code,
                command_index: Some(command_index),
            };
            match command {
                Command::Nop => {}
                Command::Font(id) => font = Some(*id),
                Command::Push => saved.push(state),
                Command::Pop => {
                    state = saved
                        .pop()
                        .ok_or_else(|| invalid("nested VF stack underflow"))?
                }
                Command::Right(n) => state.h = state.h.add(Coordinate::new(*n as i128, 20)?)?,
                Command::Down(n) => state.v = state.v.add(Coordinate::new(*n as i128, 20)?)?,
                Command::Register { register, value } => {
                    let (index, horizontal) = match register {
                        Register::W => (0, true),
                        Register::X => (1, true),
                        Register::Y => (2, false),
                        Register::Z => (3, false),
                    };
                    if let Some(value) = value {
                        state.registers[index] = *value;
                    }
                    let n = Coordinate::new(state.registers[index] as i128, 20)?;
                    if horizontal {
                        state.h = state.h.add(n)?;
                    } else {
                        state.v = state.v.add(n)?;
                    }
                }
                Command::Special(_) => {
                    return Err(crate::Error::UnsupportedFont(
                        "nested VF special handler unavailable".into(),
                    ))
                }
                Command::Rule {
                    height,
                    width,
                    advance,
                } => {
                    let w = Coordinate::new(*width as i128, 20)?;
                    let h = Coordinate::new(*height as i128, 20)?;
                    if *width > 0 && *height > 0 {
                        budget.outputs += 1;
                        if budget.outputs > 100000 {
                            return Err(invalid("nested VF output budget"));
                        }
                        placements.push(NestedPlacement::Rule {
                            x: state.h,
                            y: state.v,
                            width: w,
                            height: h,
                            source: vec![step],
                        });
                    }
                    if *advance {
                        state.h = state.h.add(w)?;
                    }
                }
                Command::Glyph { code, advance } => {
                    let code = u8::try_from(*code).map_err(|_| {
                        crate::Error::UnsupportedFont(
                            "nested VF code exceeds 8-bit TFM encoding".into(),
                        )
                    })?;
                    let id = font.ok_or_else(|| invalid("nested VF font absent"))?;
                    let child_key = fonts
                        .get(&id)
                        .ok_or_else(|| invalid("nested VF local resource missing"))?;
                    let scale = Coordinate::new(vf.fonts()[&id].scale.0 as i128, 20)?;
                    let child = self.expand_inner(child_key, code, stack, budget)?;
                    for mut placement in child.placements {
                        match &mut placement {
                            NestedPlacement::Glyph {
                                x,
                                y,
                                scale: child_scale,
                                source,
                                ..
                            } => {
                                *x = x.multiply(scale)?.add(state.h)?;
                                *y = y.multiply(scale)?.add(state.v)?;
                                *child_scale = child_scale.multiply(scale)?;
                                source.insert(0, step.clone());
                            }
                            NestedPlacement::Rule {
                                x,
                                y,
                                width,
                                height,
                                source,
                            } => {
                                *x = x.multiply(scale)?.add(state.h)?;
                                *y = y.multiply(scale)?.add(state.v)?;
                                *width = width.multiply(scale)?;
                                *height = height.multiply(scale)?;
                                source.insert(0, step.clone());
                            }
                        }
                        placements.push(placement);
                    }
                    if *advance {
                        state.h = state
                            .h
                            .add(Coordinate::new(child.width.0 as i128, 20)?.multiply(scale)?)?;
                    }
                }
            }
        }
        stack.pop();
        Ok(NestedPacket {
            resource: key.clone(),
            character: code,
            width,
            placements,
        })
    }
}
