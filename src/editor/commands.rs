use crate::document::Document;
use std::sync::Arc;

use crate::map::{LineDef, Vertex, Sector, Thing};

pub trait Command {
    fn execute(&self, document: &mut Document) -> Result<(), String>;
    fn unexecute(&self, document: &mut Document) -> Result<(), String>;
    fn undo(&self, document: &mut Document) -> Result<(), String> {
        self.unexecute(document)
    }
}

#[derive(Clone, Debug)]
pub enum CommandType {
    AddVertex {
        x: i32,
        y: i32,
        vertex_id: Option<usize>,
    },
    MoveVertex {
        vertex_id: usize,
        old_x: i32,
        old_y: i32,
        new_x: i32,
        new_y: i32,
    },
    DeleteVertex {
        vertex_id: usize,
        vertex: Option<Arc<Vertex>>,
    },
    AddLineDef {
        start_vertex_id: usize,
        end_vertex_id: usize,
        right_side_sector_id: i16,
        left_side_sector_id: i16,
        linedef_id: Option<usize>,
    },
    DeleteLineDef {
        linedef_id: usize,
        linedef: Option<Arc<LineDef>>,
    },
    AddSector {
        floor_z: i32,
        ceiling_z: i32,
        floor_texture: String,
        ceiling_texture: String,
        light_level: u8,
        sector_type: u8,
        sector_id: Option<usize>,
    },
    DeleteSector {
        sector_id: usize,
        sector: Option<Arc<Sector>>,
    },
    AddThing {
        x: i32,
        y: i32,
        angle: i32,
        thing_type: u16,
        options: u16,
        thing_id: Option<usize>,
    },
    DeleteThing {
        thing_id: usize,
        thing: Option<Arc<Thing>>,
    },
    // Add more command variants as needed (e.g., ChangeSectorProperties, etc.)
}

impl Command for Box<dyn Command> {
    fn execute(&self, document: &mut Document) -> Result<(), String> {
        (**self).execute(document)
    }
    fn unexecute(&self, document: &mut Document) -> Result<(), String> {
        (**self).unexecute(document)
    }
}

impl Command for CommandType {
    fn execute(&self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::AddVertex { x, y, vertex_id: _ } => {
                let _id = document.add_vertex(*x, *y);
                // (Consider storing `id` if you want to support unexecute.)
                Ok(())
            }
            CommandType::MoveVertex { vertex_id, new_x, new_y, .. } => {
                document.move_vertex(*vertex_id, *new_x, *new_y)
            }
            CommandType::DeleteVertex { vertex_id, vertex: _ } => {
                document
                    .remove_vertex(*vertex_id)
                    .ok_or_else(|| format!("Could not find vertex with id: {}", vertex_id))?;
                Ok(())
            }
            CommandType::AddLineDef {
                start_vertex_id,
                end_vertex_id,
                right_side_sector_id,
                left_side_sector_id,
                linedef_id: _,
            } => {
                document.add_linedef(
                    *start_vertex_id,
                    *end_vertex_id,
                    *right_side_sector_id,
                    *left_side_sector_id,
                );
                Ok(())
            }
            CommandType::DeleteLineDef { linedef_id, linedef: _ } => {
                document
                    .remove_linedef(*linedef_id)
                    .ok_or_else(|| format!("Could not find linedef with id: {}", linedef_id))?;
                Ok(())
            }
            CommandType::AddSector {
                floor_z,
                ceiling_z,
                ref floor_texture,
                ref ceiling_texture,
                light_level,
                sector_type,
                sector_id: _,
            } => {
                document.add_sector(
                    *floor_z,
                    *ceiling_z,
                    floor_texture.clone(),
                    ceiling_texture.clone(),
                    *light_level,
                    *sector_type,
                );
                Ok(())
            }
            CommandType::DeleteSector { sector_id, sector: _ } => {
                document
                    .remove_sector(*sector_id)
                    .ok_or_else(|| format!("Could not find sector with id: {}", sector_id))?;
                Ok(())
            }
            CommandType::AddThing {
                x,
                y,
                angle,
                thing_type,
                options,
                thing_id: _,
            } => {
                document.add_thing(*x, *y, *angle, *thing_type, *options);
                Ok(())
            }
            CommandType::DeleteThing { thing_id, thing: _ } => {
                document
                    .remove_thing(*thing_id)
                    .ok_or_else(|| format!("Could not find thing with id: {}", thing_id))?;
                Ok(())
            }
        }
    }

    fn unexecute(&self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::AddVertex { x: _, y: _, vertex_id } => {
                if let Some(id) = *vertex_id {
                    document.remove_vertex(id);
                    Ok(())
                } else {
                    Err("AddVertex command had no stored ID".into())
                }
            }
            CommandType::MoveVertex {
                vertex_id,
                old_x,
                old_y,
                ..
            } => document.move_vertex(*vertex_id, *old_x, *old_y),
            CommandType::DeleteVertex { vertex_id, vertex } => {
                if let Some(v) = vertex {
                    document.vertices().write().insert(*vertex_id, v.clone());
                    Ok(())
                } else {
                    Err("DeleteVertex command had no stored vertex".into())
                }
            }
            CommandType::AddLineDef {
                start_vertex_id: _,
                end_vertex_id: _,
                right_side_sector_id: _,
                left_side_sector_id: _,
                linedef_id,
            } => {
                if let Some(id) = *linedef_id {
                    document.remove_linedef(id);
                    Ok(())
                } else {
                    Err("AddLineDef command had no stored ID".into())
                }
            }
            CommandType::DeleteLineDef { linedef_id, linedef } => {
                if let Some(l) = linedef {
                    document.linedefs().write().insert(*linedef_id, l.clone());
                    Ok(())
                } else {
                    Err("DeleteLineDef command had no stored vertex".into())
                }
            }
            CommandType::AddSector {
                floor_z: _,
                ceiling_z: _,
                floor_texture: _,
                ceiling_texture: _,
                light_level: _,
                sector_type: _,
                sector_id,
            } => {
                if let Some(id) = *sector_id {
                    document.remove_sector(id);
                    Ok(())
                } else {
                    Err("AddSector command had no stored ID".into())
                }
            }
            CommandType::DeleteSector { sector_id, sector } => {
                if let Some(s) = sector {
                    document.sectors().write().insert(*sector_id, s.clone());
                    Ok(())
                } else {
                    Err("DeleteSector command had no stored vertex".into())
                }
            }
            CommandType::AddThing {
                x: _,
                y: _,
                angle: _,
                thing_type: _,
                options: _,
                thing_id,
            } => {
                if let Some(id) = *thing_id {
                    document.remove_thing(id);
                    Ok(())
                } else {
                    Err("AddThing command had no stored ID".into())
                }
            }
            CommandType::DeleteThing { thing_id, thing } => {
                if let Some(t) = thing {
                    document.things().write().insert(*thing_id, t.clone());
                    Ok(())
                } else {
                    Err("DeleteThing command had no stored vertex".into())
                }
            }
        }
    }
}

pub struct BatchCommand {
    commands: Vec<CommandType>,
}

impl BatchCommand {
    pub fn new(commands: Vec<CommandType>) -> Self {
        BatchCommand { commands }
    }
}

impl Command for BatchCommand {
    fn execute(&self, document: &mut Document) -> Result<(), String> {
        for command in &self.commands {
            command.execute(document)?;
        }
        Ok(())
    }

    fn unexecute(&self, document: &mut Document) -> Result<(), String> {
        for command in self.commands.iter().rev() {
            command.unexecute(document)?;
        }
        Ok(())
    }
}
