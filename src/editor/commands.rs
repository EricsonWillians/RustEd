// src/editor/commands.rs

use crate::document::Document;
use crate::map::{LineDef, Vertex, Sector, Thing, SideDef};
use std::sync::Arc;

pub trait Command {
    fn execute(&mut self, document: &mut Document) -> Result<(), String>;
    fn unexecute(&mut self, document: &mut Document) -> Result<(), String>;
    fn undo(&mut self, document: &mut Document) -> Result<(), String> {
        self.unexecute(document)
    }
}

#[derive(Clone, Debug)]
pub enum CommandType {
    // Vertex Commands
    AddVertex {
        x: i32,
        y: i32,
        vertex_id: Option<usize>,
    },
    MoveVertex {
        vertex_id: usize,
        dx: i32,
        dy: i32,
    },
    DeleteVertex {
        vertex_id: usize,
        vertex: Option<Arc<Vertex>>,
    },

    // LineDef Commands
    AddLineDef {
        start: usize,
        end: usize,
        flags: i32,
        line_type: i32,
        tag: i32,
        right: i32,
        left: i32,
        linedef_id: Option<usize>,
    },
    DeleteLineDef {
        linedef_id: usize,
        linedef: Option<Arc<LineDef>>,
    },
    ModifyLineDef {
        linedef_id: usize,
        flags: Option<i32>,
        line_type: Option<i32>,
        tag: Option<i32>,
    },
    MakeLineTwoSided {
        linedef_id: usize,
        sector_id: i32,
    },

    // SideDef Commands
    AddSideDef {
        x_offset: i32,
        y_offset: i32,
        upper_tex: String,
        lower_tex: String,
        mid_tex: String,
        sector: i32,
        sidedef_id: Option<usize>,
    },
    DeleteSideDef {
        sidedef_id: usize,
        sidedef: Option<Arc<SideDef>>,
    },
    ModifySideDef {
        sidedef_id: usize,
        x_offset: Option<i32>,
        y_offset: Option<i32>,
        upper_tex: Option<String>,
        lower_tex: Option<String>,
        mid_tex: Option<String>,
    },

    // Sector Commands
    AddSector {
        floor_height: i32,
        ceiling_height: i32,
        floor_tex: String,
        ceiling_tex: String,
        light: i32,
        r#type: i32,
        tag: i32,
        sector_id: Option<usize>,
    },
    DeleteSector {
        sector_id: usize,
        sector: Option<Arc<Sector>>,
    },
    ModifySector {
        sector_id: usize,
        floor_height: Option<i32>,
        ceiling_height: Option<i32>,
        floor_tex: Option<String>,
        ceiling_tex: Option<String>,
        light: Option<i32>,
        r#type: Option<i32>,
        tag: Option<i32>,
    },
    MergeSectors {
        sector_a: usize,
        sector_b: usize,
        target_properties: SectorProperties,
    },

    // Thing Commands
    AddThing {
        x: i32,
        y: i32,
        angle: i32,
        doom_type: i32,
        flags: i32,
        thing_id: Option<usize>,
    },
    DeleteThing {
        thing_id: usize,
        thing: Option<Arc<Thing>>,
    },
    MoveThing {
        thing_id: usize,
        dx: i32,
        dy: i32,
    },
    RotateThing {
        thing_id: usize,
        new_angle: i32,
    },
    ModifyThing {
        thing_id: usize,
        doom_type: Option<i32>,
        flags: Option<i32>,
    },

    // Batch Operations
    BatchCommand { commands: Vec<CommandType> },
}

#[derive(Clone, Debug)]
pub struct SectorProperties {
    pub floor_height: i32,
    pub ceiling_height: i32,
    pub floor_tex: String,
    pub ceiling_tex: String,
    pub light: i32,
    pub r#type: i32,
    pub tag: i32,
}

impl Command for CommandType {
    fn execute(&mut self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::BatchCommand { ref mut commands } => {
                for command in commands {
                    command.execute(document)?;
                }
                Ok(())
            }
            CommandType::AddVertex { x, y, ref mut vertex_id } => {
                let id = document.add_vertex(*x, *y);
                *vertex_id = Some(id);
                Ok(())
            }
            CommandType::MoveVertex { vertex_id, dx, dy } => {
                let vertices_arc = document.vertices();
                let mut vertices = vertices_arc.write();
                if let Some(vertex) = vertices.get_mut(*vertex_id) {
                    let vertex = Arc::make_mut(vertex);
                    vertex.x += *dx;
                    vertex.y += *dy;
                    Ok(())
                } else {
                    Err(format!("Vertex {} not found", vertex_id))
                }
            }
            CommandType::DeleteVertex { vertex_id, vertex } => {
                let vertices_arc = document.vertices();
                let mut vertices = vertices_arc.write();
                if *vertex_id < vertices.len() {
                    let v = vertices.remove(*vertex_id);
                    *vertex = Some(v);
                    Ok(())
                } else {
                    Err(format!("Vertex {} not found", vertex_id))
                }
            }
            CommandType::AddLineDef { 
                start, end, flags, line_type, tag, right, left, 
                ref mut linedef_id 
            } => {
                let linedef = LineDef {
                    start: *start,
                    end: *end,
                    flags: *flags,
                    line_type: *line_type,
                    tag: *tag,
                    right: *right,
                    left: *left,
                };
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                linedefs.push(Arc::new(linedef));
                *linedef_id = Some(linedefs.len() - 1);
                Ok(())
            }
            CommandType::DeleteLineDef { linedef_id, linedef } => {
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                if *linedef_id < linedefs.len() {
                    let ld = linedefs.remove(*linedef_id);
                    *linedef = Some(ld);
                    Ok(())
                } else {
                    Err(format!("LineDef {} not found", linedef_id))
                }
            }
            CommandType::ModifyLineDef { linedef_id, flags, line_type, tag } => {
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                if let Some(linedef) = linedefs.get_mut(*linedef_id) {
                    let linedef = Arc::make_mut(linedef);
                    if let Some(f) = flags {
                        linedef.flags = *f;
                    }
                    if let Some(t) = line_type {
                        linedef.line_type = *t;
                    }
                    if let Some(t) = tag {
                        linedef.tag = *t;
                    }
                    Ok(())
                } else {
                    Err(format!("LineDef {} not found", linedef_id))
                }
            }
            CommandType::MakeLineTwoSided { linedef_id, sector_id } => {
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                if let Some(linedef) = linedefs.get_mut(*linedef_id) {
                    let linedef = Arc::make_mut(linedef);
                    linedef.flags |= 0x0004;
                    linedef.left = *sector_id;
                    Ok(())
                } else {
                    Err(format!("LineDef {} not found", linedef_id))
                }
            }
            CommandType::AddSideDef {
                x_offset, y_offset, upper_tex, lower_tex, mid_tex,
                sector, ref mut sidedef_id
            } => {
                let sidedef = SideDef {
                    x_offset: *x_offset,
                    y_offset: *y_offset,
                    upper_tex: upper_tex.clone(),
                    lower_tex: lower_tex.clone(),
                    mid_tex: mid_tex.clone(),
                    sector: *sector,
                };
                let sidedefs_arc = document.sidedefs();
                let mut sidedefs = sidedefs_arc.write();
                sidedefs.push(Arc::new(sidedef));
                *sidedef_id = Some(sidedefs.len() - 1);
                Ok(())
            }
            CommandType::DeleteSideDef { sidedef_id, sidedef } => {
                let sidedefs_arc = document.sidedefs();
                let mut sidedefs = sidedefs_arc.write();
                if *sidedef_id < sidedefs.len() {
                    let sd = sidedefs.remove(*sidedef_id);
                    *sidedef = Some(sd);
                    Ok(())
                } else {
                    Err(format!("SideDef {} not found", sidedef_id))
                }
            }
            CommandType::ModifySideDef {
                sidedef_id,
                x_offset,
                y_offset,
                upper_tex,
                lower_tex,
                mid_tex,
            } => {
                let sidedefs_arc = document.sidedefs();
                let mut sidedefs = sidedefs_arc.write();
                if let Some(sidedef) = sidedefs.get_mut(*sidedef_id) {
                    let sidedef = Arc::make_mut(sidedef);
                    if let Some(x) = x_offset {
                        sidedef.x_offset = *x;
                    }
                    if let Some(y) = y_offset {
                        sidedef.y_offset = *y;
                    }
                    if let Some(tex) = upper_tex {
                        sidedef.upper_tex = tex.clone();
                    }
                    if let Some(tex) = lower_tex {
                        sidedef.lower_tex = tex.clone();
                    }
                    if let Some(tex) = mid_tex {
                        sidedef.mid_tex = tex.clone();
                    }
                    Ok(())
                } else {
                    Err(format!("SideDef {} not found", sidedef_id))
                }
            }
            CommandType::AddSector {
                floor_height,
                ceiling_height,
                floor_tex,
                ceiling_tex,
                light,
                r#type,
                tag,
                ref mut sector_id
            } => {
                let sector = Sector {
                    floor_height: *floor_height,
                    ceiling_height: *ceiling_height,
                    floor_tex: floor_tex.clone(),
                    ceiling_tex: ceiling_tex.clone(),
                    light: *light,
                    r#type: *r#type,
                    tag: *tag,
                };
                let sectors_arc = document.sectors();
                let mut sectors = sectors_arc.write();
                sectors.push(Arc::new(sector));
                *sector_id = Some(sectors.len() - 1);
                Ok(())
            }
            CommandType::DeleteSector { sector_id, sector } => {
                let sectors_arc = document.sectors();
                let mut sectors = sectors_arc.write();
                if *sector_id < sectors.len() {
                    let sec = sectors.remove(*sector_id);
                    *sector = Some(sec);
                    Ok(())
                } else {
                    Err(format!("Sector {} not found", sector_id))
                }
            }
            CommandType::ModifySector {
                sector_id,
                floor_height,
                ceiling_height,
                floor_tex,
                ceiling_tex,
                light,
                r#type,
                tag,
            } => {
                let sectors_arc = document.sectors();
                let mut sectors = sectors_arc.write();
                if let Some(sector) = sectors.get_mut(*sector_id) {
                    let sector = Arc::make_mut(sector);
                    if let Some(h) = floor_height {
                        sector.floor_height = *h;
                    }
                    if let Some(h) = ceiling_height {
                        sector.ceiling_height = *h;
                    }
                    if let Some(tex) = floor_tex {
                        sector.floor_tex = tex.clone();
                    }
                    if let Some(tex) = ceiling_tex {
                        sector.ceiling_tex = tex.clone();
                    }
                    if let Some(l) = light {
                        sector.light = *l;
                    }
                    if let Some(t) = r#type {
                        sector.r#type = *t;
                    }
                    if let Some(t) = tag {
                        sector.tag = *t;
                    }
                    Ok(())
                } else {
                    Err(format!("Sector {} not found", sector_id))
                }
            }
            CommandType::MergeSectors {
                sector_a,
                sector_b,
                target_properties,
            } => {
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                for linedef in linedefs.iter_mut() {
                    let linedef = Arc::make_mut(linedef);
                    if linedef.right == *sector_b as i32 {
                        linedef.right = *sector_a as i32;
                    }
                    if linedef.left == *sector_b as i32 {
                        linedef.left = *sector_a as i32;
                    }
                }

                let sectors_arc = document.sectors();
                let mut sectors = sectors_arc.write();
                if let Some(sector) = sectors.get_mut(*sector_a) {
                    let sector = Arc::make_mut(sector);
                    sector.floor_height = target_properties.floor_height;
                    sector.ceiling_height = target_properties.ceiling_height;
                    sector.floor_tex = target_properties.floor_tex.clone();
                    sector.ceiling_tex = target_properties.ceiling_tex.clone();
                    sector.light = target_properties.light;
                    sector.r#type = target_properties.r#type;
                    sector.tag = target_properties.tag;
                }

                sectors.remove(*sector_b);
                Ok(())
            }
            CommandType::AddThing {
                x,
                y,
                angle,
                doom_type,
                flags,
                ref mut thing_id
            } => {
                let thing = Thing {
                    x: *x,
                    y: *y,
                    angle: *angle,
                    doom_type: *doom_type,
                    flags: *flags,
                };
                let things_arc = document.things();
                let mut things = things_arc.write();
                things.push(Arc::new(thing));
                *thing_id = Some(things.len() - 1);
                Ok(())
            }
            CommandType::DeleteThing { thing_id, thing } => {
                let things_arc = document.things();
                let mut things = things_arc.write();
                if *thing_id < things.len() {
                    let t = things.remove(*thing_id);
                    *thing = Some(t);
                    Ok(())
                } else {
                    Err(format!("Thing {} not found", thing_id))
                }
            }
            CommandType::MoveThing { thing_id, dx, dy } => {
                let things_arc = document.things();
                let mut things = things_arc.write();
                if let Some(thing) = things.get_mut(*thing_id) {
                    let thing = Arc::make_mut(thing);
                    thing.x += *dx;
                    thing.y += *dy;
                    Ok(())
                } else {
                    Err(format!("Thing {} not found", thing_id))
                }
            }
            CommandType::RotateThing { thing_id, new_angle } => {
                let things_arc = document.things();
                let mut things = things_arc.write();
                if let Some(thing) = things.get_mut(*thing_id) {
                    let thing = Arc::make_mut(thing);
                    thing.angle = *new_angle;
                    Ok(())
                } else {
                    Err(format!("Thing {} not found", thing_id))
                }
            }
            CommandType::ModifyThing { thing_id, doom_type, flags } => {
                let things_arc = document.things();
                let mut things = things_arc.write();
                if let Some(thing) = things.get_mut(*thing_id) {
                    let thing = Arc::make_mut(thing);
                    if let Some(t) = doom_type {
                        thing.doom_type = *t;
                    }
                    if let Some(f) = flags {
                        thing.flags = *f;
                    }
                    Ok(())
                } else {
                    Err(format!("Thing {} not found", thing_id))
                }
            }
        }
    }

    fn unexecute(&mut self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::BatchCommand { commands } => {
                for command in commands.iter_mut().rev() {
                    command.unexecute(document)?;
                }
                Ok(())
            }
            CommandType::AddVertex { vertex_id, .. } => {
                if let Some(id) = vertex_id {
                    let vertices_arc = document.vertices();
                    vertices_arc.write().remove(*id);
                    Ok(())
                } else {
                    Err("No vertex ID stored for undo".into())
                }
            }
            CommandType::MoveVertex { vertex_id, dx, dy } => {
                let vertices_arc = document.vertices();
                let mut vertices = vertices_arc.write();
                if let Some(vertex) = vertices.get_mut(*vertex_id) {
                    let vertex = Arc::make_mut(vertex);
                    vertex.x -= *dx;
                    vertex.y -= *dy;
                    Ok(())
                } else {
                    Err(format!("Vertex {} not found for undo", vertex_id))
                }
            }
            CommandType::DeleteVertex { vertex_id, vertex } => {
                if let Some(v) = vertex {
                    let vertices_arc = document.vertices();
                    vertices_arc.write().insert(*vertex_id, v.clone());
                    Ok(())
                } else {
                    Err("No vertex data stored for undo".into())
                }
            }
            CommandType::AddLineDef { linedef_id, .. } => {
                if let Some(id) = linedef_id {
                    let linedefs_arc = document.linedefs();
                    linedefs_arc.write().remove(*id);
                    Ok(())
                } else {
                    Err("No linedef ID stored for undo".into())
                }
            }
            CommandType::DeleteLineDef { linedef_id, linedef } => {
                if let Some(l) = linedef {
                    let linedefs_arc = document.linedefs();
                    linedefs_arc.write().insert(*linedef_id, l.clone());
                    Ok(())
                } else {
                    Err("No linedef data stored for undo".into())
                }
            }
            CommandType::ModifyLineDef { linedef_id, .. } => {
                Err("ModifyLineDef undo not implemented - old values not stored".into())
            }
            CommandType::MakeLineTwoSided { linedef_id, sector_id: _ } => {
                let linedefs_arc = document.linedefs();
                let mut linedefs = linedefs_arc.write();
                if let Some(linedef) = linedefs.get_mut(*linedef_id) {
                    let linedef = Arc::make_mut(linedef);
                    linedef.flags &= !0x0004;
                    linedef.left = -1;
                    Ok(())
                } else {
                    Err(format!("LineDef {} not found for undo", linedef_id))
                }
            }
            CommandType::AddSideDef { sidedef_id, .. } => {
                if let Some(id) = sidedef_id {
                    let sidedefs_arc = document.sidedefs();
                    sidedefs_arc.write().remove(*id);
                    Ok(())
                } else {
                    Err("No sidedef ID stored for undo".into())
                }
            }
            CommandType::DeleteSideDef { sidedef_id, sidedef } => {
                if let Some(s) = sidedef {
                    let sidedefs_arc = document.sidedefs();
                    sidedefs_arc.write().insert(*sidedef_id, s.clone());
                    Ok(())
                } else {
                    Err("No sidedef data stored for undo".into())
                }
            }
            CommandType::ModifySideDef { sidedef_id, .. } => {
                Err("ModifySideDef undo not implemented - old values not stored".into())
            }
            CommandType::AddSector { sector_id, .. } => {
                if let Some(id) = sector_id {
                    let sectors_arc = document.sectors();
                    sectors_arc.write().remove(*id);
                    Ok(())
                } else {
                    Err("No sector ID stored for undo".into())
                }
            }
            CommandType::DeleteSector { sector_id, sector } => {
                if let Some(s) = sector {
                    let sectors_arc = document.sectors();
                    sectors_arc.write().insert(*sector_id, s.clone());
                    Ok(())
                } else {
                    Err("No sector data stored for undo".into())
                }
            }
            CommandType::ModifySector { sector_id, .. } => {
                Err("ModifySector undo not implemented - old values not stored".into())
            }
            CommandType::MergeSectors { sector_a: _, sector_b: _, target_properties: _ } => {
                Err("MergeSectors undo not implemented - need to store original state".into())
            }
            CommandType::AddThing { thing_id, .. } => {
                if let Some(id) = thing_id {
                    let things_arc = document.things();
                    things_arc.write().remove(*id);
                    Ok(())
                } else {
                    Err("No thing ID stored for undo".into())
                }
            }
            CommandType::DeleteThing { thing_id, thing } => {
                if let Some(t) = thing {
                    let things_arc = document.things();
                    things_arc.write().insert(*thing_id, t.clone());
                    Ok(())
                } else {
                    Err("No thing data stored for undo".into())
                }
            }
            CommandType::MoveThing { thing_id, dx, dy } => {
                let things_arc = document.things();
                let mut things = things_arc.write();
                if let Some(thing) = things.get_mut(*thing_id) {
                    let thing = Arc::make_mut(thing);
                    thing.x -= *dx;
                    thing.y -= *dy;
                    Ok(())
                } else {
                    Err(format!("Thing {} not found for undo", thing_id))
                }
            }
            CommandType::RotateThing { thing_id, .. } => {
                Err("RotateThing undo not implemented - old angle not stored".into())
            }
            CommandType::ModifyThing { thing_id, .. } => {
                Err("ModifyThing undo not implemented - old values not stored".into())
            }
        }
    }
}

impl Command for Box<dyn Command> {
    fn execute(&mut self, document: &mut Document) -> Result<(), String> {
        (**self).execute(document)
    }
    fn unexecute(&mut self, document: &mut Document) -> Result<(), String> {
        (**self).unexecute(document)
    }
}