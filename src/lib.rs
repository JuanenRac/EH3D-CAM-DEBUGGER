// EH3D-CAM-DEBUGGER - checks a G-code program before it reaches a machine.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Reads a G-code program with the interpreter of EH3D-CORE-LIBS, works out how long it takes with the acceleration of the machine and looks for what would hurt:
//! a rapid move into the material, a move outside the table, a cut with the spindle off, a feed that is too fast or a head that passes over a clamp.

pub mod checks;
pub mod timeline;
