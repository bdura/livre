//! Text state.
//!
//! > At the beginning of a text object, T m shall be the identity matrix;
//! > therefore, the origin of text space shall be initially the same as that of
//! > user space. The text-positioning operators, described in
//! > "Table 106 — Text-positioning operators" alter T m and thereby control the
//! > placement of glyphs that are subsequently painted. Also, the text-showing
//! > operators, described in "Table 107 — Text-showing operators", update T m
//! > (by altering its e and f translation components) to take into account
//! > the horizontal or vertical displacement of each glyph painted as well as
//! > any character or word-spacing parameters in the text state.
