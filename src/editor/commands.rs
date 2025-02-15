// src/editor/commands.rs
use crate::document::{Document, Vertex, LineDef, Sector, Thing};

// Trait object with the changes.
pub trait Command {
    fn execute(&mut self, document: &mut Document) -> Result<(), String>;
    fn unexecute(&mut self, document: &mut Document) -> Result<(), String>;
}


#[derive(Clone, Debug)] // Allow cloning and printing for debugging
pub enum CommandType { //Rename the commands
    AddVertex {
        x: i32,
        y: i32,
        vertex_id: Option<usize>, // Store the ID *after* the vertex is added
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
        vertex: Option<Vertex> // Store the vertex *before* deleting
    },
    AddLineDef{
        start_vertex_id: usize,
        end_vertex_id: usize,
        right_side_sector_id: i16,
        left_side_sector_id: i16,
        linedef_id: Option<usize>
    },
    DeleteLineDef{
        linedef_id: usize,
        linedef: Option<LineDef>
    },
    AddSector{
        floor_z: i32,
        ceiling_z: i32,
        floor_texture: String,
        ceiling_texture: String,
        light_level: u8,
        sector_type: u8,
        sector_id: Option<usize>
    },
     DeleteSector{
        sector_id: usize,
        sector: Option<Sector>
    },
    AddThing{
        x: i32,
        y: i32,
        angle: i32,
        thing_type: u16,
        options: u16,
        thing_id: Option<usize>
    },
    DeleteThing{
        thing_id: usize,
        thing: Option<Thing>
    }
    // Add more command variants as needed (e.g., ChangeSectorProperties, etc.)
}

impl Command for CommandType{
     fn execute(&mut self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::AddVertex { x, y, vertex_id } => {
                let id = document.add_vertex(*x, *y);
                // Store the assigned ID. Very important for undo!
                if let CommandType::AddVertex { vertex_id: ref mut stored_id, .. } = self {
                    *stored_id = Some(id);
                }
                Ok(())
            }
            CommandType::MoveVertex { vertex_id, new_x, new_y, .. } => {
                document.move_vertex(*vertex_id, *new_x, *new_y)
            }
            CommandType::DeleteVertex{ vertex_id, vertex } =>{
                if let Some(v) = document.remove_vertex(*vertex_id){
                    if let CommandType::DeleteVertex { vertex: ref mut stored_vertex, .. } = self{
                        *stored_vertex = Some(v);
                    }
                    Ok(())
                }
                else{
                    Err(format!("Could not find vertex with id: {}", vertex_id))
                }
            }
            CommandType::AddLineDef {
                start_vertex_id,
                end_vertex_id,
                right_side_sector_id,
                left_side_sector_id,
                linedef_id
            } => {
                 let id = document.add_linedef(*start_vertex_id, *end_vertex_id, *right_side_sector_id, *left_side_sector_id);
                if let CommandType::AddLineDef { linedef_id: ref mut stored_id, .. } = self {
                    *stored_id = Some(id);
                }
                 Ok(())
            }
            CommandType::DeleteLineDef{ linedef_id, linedef } =>{
                if let Some(l) = document.remove_linedef(*linedef_id){
                    if let CommandType::DeleteLineDef { linedef: ref mut stored_linedef, .. } = self{
                        *stored_linedef = Some(l);
                    }
                    Ok(())
                }
                else{
                    Err(format!("Could not find linedef with id: {}", linedef_id))
                }
            }
            CommandType::AddSector{
                floor_z,
                ceiling_z,
                floor_texture,
                ceiling_texture,
                light_level,
                sector_type,
                sector_id
            } => {
                 let id = document.add_sector(*floor_z,*ceiling_z, floor_texture.clone(), ceiling_texture.clone(), *light_level, *sector_type);
                if let CommandType::AddSector { sector_id: ref mut stored_id, .. } = self {
                    *stored_id = Some(id);
                }
                 Ok(())
            }
            CommandType::DeleteSector{ sector_id, sector } =>{
                if let Some(s) = document.remove_sector(*sector_id){
                    if let CommandType::DeleteSector { sector: ref mut stored_sector, .. } = self{
                        *stored_sector = Some(s);
                    }
                    Ok(())
                }
                else{
                    Err(format!("Could not find sector with id: {}", sector_id))
                }
            }
            CommandType::AddThing{
                x,
                y,
                angle,
                thing_type,
                options,
                thing_id
            } => {
                 let id = document.add_thing(*x, *y, *angle, *thing_type, *options);
                if let CommandType::AddThing { thing_id: ref mut stored_id, .. } = self {
                    *stored_id = Some(id);
                }
                 Ok(())
            }
            CommandType::DeleteThing{ thing_id, thing } =>{
                if let Some(t) = document.remove_thing(*thing_id){
                    if let CommandType::DeleteThing { thing: ref mut stored_thing, .. } = self{
                        *stored_thing = Some(t);
                    }
                    Ok(())
                }
                else{
                    Err(format!("Could not find thing with id: {}", thing_id))
                }
            }
        }
    }


     fn unexecute(&mut self, document: &mut Document) -> Result<(), String> {
        match self {
            CommandType::AddVertex { x, y, vertex_id } => {
                // Now we use the stored ID to remove the vertex.
                if let Some(id) = vertex_id {
                    document.remove_vertex(*id);
                    Ok(())
                } else {
                    Err("AddVertex command had no stored ID".into()) // This should never happen if execute() is working correctly.
                }
            }
            CommandType::MoveVertex { vertex_id, old_x, old_y, .. } => {
                // Move the vertex back to its old position
                document.move_vertex(*vertex_id, *old_x, *old_y)
            }
            CommandType::DeleteVertex { vertex_id, vertex } => {
                if let Some(v) = vertex{
                    document.vertices().write().insert(*vertex_id, std::sync::Arc::new(v.clone()));
                    Ok(())
                }
                else{
                    Err(format!("DeleteVertex command had no stored vertex"))
                }
            }
             CommandType::AddLineDef {
                start_vertex_id,
                end_vertex_id,
                right_side_sector_id,
                left_side_sector_id,
                linedef_id
            } => {
                 if let Some(id) = linedef_id {
                    document.remove_linedef(*id);
                    Ok(())
                } else {
                    Err("AddLineDef command had no stored ID".into()) // This should never happen if execute() is working correctly.
                }
            }
            CommandType::DeleteLineDef{ linedef_id, linedef } => {
                if let Some(l) = linedef{
                    document.linedefs().write().insert(*linedef_id, std::sync::Arc::new(l.clone()));
                    Ok(())
                }
                else{
                    Err(format!("DeleteLineDef command had no stored vertex"))
                }
            }
            CommandType::AddSector{
                floor_z,
                ceiling_z,
                floor_texture,
                ceiling_texture,
                light_level,
                sector_type,
                sector_id
            } => {
                if let Some(id) = sector_id {
                   document.remove_sector(*id);
                   Ok(())
               } else {
                   Err("AddSector command had no stored ID".into()) // This should never happen if execute() is working correctly.
               }
            }
            CommandType::DeleteSector{ sector_id, sector } => {
               if let Some(s) = sector{
                   document.sectors().write().insert(*sector_id, std::sync::Arc::new(s.clone()));
                   Ok(())
               }
               else{
                   Err(format!("DeleteSector command had no stored vertex"))
               }
           }
           CommandType::AddThing{
                x,
                y,
                angle,
                thing_type,
                options,
                thing_id
            } => {
                if let Some(id) = thing_id {
                  document.remove_thing(*id);
                  Ok(())
              } else {
                  Err("AddThing command had no stored ID".into()) // This should never happen if execute() is working correctly.
              }
           }
           CommandType::DeleteThing{ thing_id, thing } => {
              if let Some(t) = thing{
                  document.things().write().insert(*thing_id, std::sync::Arc::new(t.clone()));
                  Ok(())
              }
              else{
                  Err(format!("DeleteThing command had no stored vertex"))
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

impl Command for BatchCommand{
     fn execute(&mut self, document: &mut Document) -> Result<(), String>{
        for command in &mut self.commands {
            command.execute(document)?;
        }
        Ok(())
    }

     fn unexecute(&mut self, document: &mut Document) -> Result<(), String>{
        // Undo commands in reverse order
        for command in self.commands.iter_mut().rev() {
            command.unexecute(document)?;
        }
        Ok(())
    }
}