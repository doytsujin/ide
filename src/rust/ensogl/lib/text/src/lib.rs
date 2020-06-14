//! Ensogl text rendering implementation.

#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unsafe_code)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]

pub mod glyph;
pub mod model;

/// Commonly used types and functions.
pub mod prelude {
    pub use ensogl::prelude::*;
}

pub use ensogl::display;

use model::*;

use selection::Selection;

//use prelude::*;
//
//

use xi_rope as rope;

use rope::spans::Spans;
use rope::spans::SpansBuilder;
use rope::breaks::{BreakBuilder, Breaks, BreaksInfo, BreaksMetric};
use rope::{Interval, LinesMetric, Rope, RopeDelta, RopeInfo};
//use rope::LinesMetric;
//use rope::rope::BaseMetric;
//use rope::tree::*;
//
//
//
//
//
//
//pub struct Line {
//    text  : Rope,
//    index : usize,
//}

use std::cmp::max;
use std::cmp::min;



pub fn main() {
    let buffer = Buffer::from("Test text!");
    buffer.set_color(1..3,color::Rgba::new(1.0,0.0,0.0,1.0));
    let mut view = buffer.view();


//    let foo = buffer.color.iter().collect_vec();
    let foo = buffer.color.borrow().subseq(2..5);
    let foo = foo.iter().collect_vec();
    println!("{:#?}",foo);

    println!("{:#?}",view.selection);

    view.move_selection(Movement::Right,false);

    println!("{:#?}",view.selection);
}



#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash)]
pub struct BufferId(pub usize);

pub struct BufferMap {
    map : BTreeMap<BufferId,Buffer>
}



// ==============
// === Buffer ===
// ==============

impl_clone_ref_as_clone!(Buffer);
#[derive(Clone,Debug,Default)]
pub struct Buffer {
    /// The contents of the buffer.
    pub rope: Rope,

    pub color : Rc<RefCell<Spans<color::Rgba>>>,
}


impl Buffer {
    pub fn new() -> Self {
        default()
    }

    pub fn set_color(&self, interval:impl Into<Interval>, color:impl Into<color::Rgba>) {
        let interval = interval.into();
        let color    = color.into();

        let mut sb = SpansBuilder::new(interval.end());
        sb.add_span(interval,color);

        self.color.borrow_mut().edit(interval,sb.build());
    }

    pub fn view(&self) -> View {
        View::new(self)
    }
}


// === Conversions ===

impl From<Rope> for Buffer {
    fn from(rope:Rope) -> Self {
        Self {rope,..default()}
    }
}

impl From<&Rope> for Buffer {
    fn from(rope:&Rope) -> Self {
        let rope = rope.clone();
        Self {rope,..default()}
    }
}

impl From<&str> for Buffer {
    fn from(s:&str) -> Self {
        Rope::from(s).into()
    }
}

impl From<String> for Buffer {
    fn from(s:String) -> Self {
        Rope::from(s).into()
    }
}

impl From<&String> for Buffer {
    fn from(s:&String) -> Self {
        Rope::from(s).into()
    }
}

impl From<&&String> for Buffer {
    fn from(s:&&String) -> Self {
        (*s).into()
    }
}

impl From<&&str> for Buffer {
    fn from(s:&&str) -> Self {
        (*s).into()
    }
}



// ============
// === View ===
// ============

pub struct View {
    buffer : Buffer,
    /// vertical scroll position
    first_line: usize,
    /// height of visible portion
    height: usize,
    selection: SelectionGroup,
    /// New offset to be scrolled into position after an edit.
    scroll_to: Option<usize>,
}


impl LineOffset for View {
    fn text(&self) -> &Rope {
        &self.buffer.rope
    }

    fn offset_of_line(&self,line:usize) -> usize {
        let line = line.min(self.text().measure::<LinesMetric>() + 1);
        self.text().offset_of_line(line)
    }

    fn line_of_offset(&self,offset:usize) -> usize {
        self.text().line_of_offset(offset)
    }
}


/// When paging through a file, the number of lines from the previous page
/// that will also be visible in the next.
const SCROLL_OVERLAP: isize = 2;

impl View {
    fn new(buffer:impl Into<Buffer>) -> Self {
        let buffer = buffer.into();
        let first_line = 0;
        let height = 10;
        let mut selection = SelectionGroup::default();
        let scroll_to = None;
        selection.regions.push(Selection::new(0,0));
        Self {buffer,first_line,height,selection,scroll_to}
    }

    /// If `modify` is `true`, the selections are modified, otherwise the results
    /// of individual region movements become carets.
    pub fn move_selection(&mut self, movement: Movement, modify: bool) {
        self.set_selection(self.moved_selection(movement,modify));
    }
//
//    pub fn scroll_height(&self) -> usize {
//        self.height
//    }
//
    /// Computes the actual desired amount of scrolling (generally slightly
    /// less than the height of the viewport, to allow overlap).
    fn page_scroll_height(&self) -> isize {
        max(self.height as isize - SCROLL_OVERLAP, 1)
    }

    /// Returns the regions of the current selection.
    pub fn sel_regions(&self) -> &[Selection] {
        &self.selection
    }

    /// Set the selection to a new value.
    pub fn set_selection(&mut self, selection:impl Into<SelectionGroup>) {
        //self.invalidate_selection();
        self.selection = selection.into();
        //self.invalidate_selection();
//        self.scroll_to_cursor(text);
    }

    /// Sets the selection to a new value, invalidating the line cache as needed.
    /// This function does not perform any scrolling.
    fn set_selection_raw(&mut self, sel: SelectionGroup) {

    }

//    fn scroll_to_cursor(&mut self, text: &Rope) {
//        let end = self.sel_regions().last().unwrap().end;
//        let line = self.line_of_offset(text, end);
//        if line < self.first_line {
//            self.first_line = line;
//        } else if self.first_line + self.height <= line {
//            self.first_line = line - (self.height - 1);
//        }
//        // We somewhat arbitrarily choose the last region for setting the old-style
//        // selection state, and for scrolling it into view if needed. This choice can
//        // likely be improved.
//        self.scroll_to = Some(end);
//    }

    /// Invalidate the current selection. Note that we could be even more
    /// fine-grained in the case of multiple cursors, but we also want this
    /// method to be fast even when the selection is large.
    fn invalidate_selection(&mut self, text: &Rope) {
//        // TODO: refine for upstream (caret appears on prev line)
//        let first_line = self.line_of_offset(text, self.selection.first().unwrap().min());
//        let last_line = self.line_of_offset(text, self.selection.last().unwrap().max()) + 1;
//        let all_caret = self.selection.iter().all(|region| region.is_caret());
//        let invalid = if all_caret {
//            line_cache_shadow::CURSOR_VALID
//        } else {
//            line_cache_shadow::CURSOR_VALID | line_cache_shadow::STYLES_VALID
//        };
//        self.lc_shadow.partial_invalidate(first_line, last_line, invalid);
    }
}


impl View {
    /// Based on the current selection position this will return the cursor position, the current line, and the
/// total number of lines of the file.
    fn selection_position(&self,
        r: Selection,
        move_up: bool,
        modify: bool,
    ) -> (location::Column, usize) {
        // The active point of the selection
        let active = if modify {
            r.end
        } else if move_up {
            r.min()
        } else {
            r.max()
        };
        let col = if let Some(col) = r.horiz { col } else { self.offset_to_line_col(active).1.into() };
        let line = self.line_of_offset(active);

        (col, line)
    }


    /// Compute movement based on vertical motion by the given number of lines.
///
/// Note: in non-exceptional cases, this function preserves the `horiz`
/// field of the selection region.
    fn vertical_motion(&self,
        region: Selection,
        line_delta: isize,
        modify: bool,
    ) -> (usize, Option<location::Column>) {
        let (col, line) = self.selection_position(region, line_delta < 0, modify);
        let n_lines = self.line_of_offset(self.text().len());

        // This code is quite careful to avoid integer overflow.
        // TODO: write tests to verify
        if line_delta < 0 && (-line_delta as usize) > line {
            return (0, Some(col));
        }
        let line = if line_delta < 0 {
            line - (-line_delta as usize)
        } else {
            line.saturating_add(line_delta as usize)
        };
        if line > n_lines {
            return (self.text().len(), Some(col));
        }
        let new_offset = self.line_col_to_offset(line, col.into());
        (new_offset, Some(col))
    }

    /// Compute movement based on vertical motion by the given number of lines skipping
/// any line that is shorter than the current cursor position.
    fn vertical_motion_exact_pos(&self,
        region: Selection,
        move_up: bool,
        modify: bool,
    ) -> (usize, Option<location::Column>) {
        let (col, init_line) = self.selection_position(region, move_up, modify);
        let n_lines = self.line_of_offset(self.text().len());

        let mut line_length =
            self.offset_of_line(init_line.saturating_add(1)) - self.offset_of_line(init_line);
        if move_up && init_line == 0 {
            return (self.line_col_to_offset(init_line, col.into()), Some(col));
        }
        let mut line = if move_up { init_line - 1 } else { init_line.saturating_add(1) };

        // If the active columns is longer than the current line, use the current line length.
        let line_last_column = location::Column(line_length);
        let col = if line_last_column < col { line_last_column - 1 } else { col };

        loop {
            let line_len = self.offset_of_line(line + 1) - self.offset_of_line(line);

            // If the line is longer than the current cursor position, break.
            // We use > instead of >= because line_length includes newline.
            if line_len > col.raw {
                break;
            }

            // If you are trying to add a selection past the end of the file or before the first line, return original selection
            if line >= n_lines || (line == 0 && move_up) {
                line = init_line;
                break;
            }

            line = if move_up { line - 1 } else { line.saturating_add(1) };
        }

        (self.line_col_to_offset(line, col), Some(col))
    }
}





use Movement::*;

impl View {
    /// Apply the movement to each region in the selection, and returns the union of the results.
    ///
    /// If `modify` is `true`, the selections are modified, otherwise the results of individual region
    /// movements become carets. Modify is often mapped to the `shift` button in text editors.
    pub fn moved_selection(&self, movement:Movement, modify:bool) -> SelectionGroup {
        let mut result = SelectionGroup::new();
        for &region in self.selection.iter() {
            let new_region = self.moved_selection_region(movement,region,modify);
            result.add_region(new_region);
        }
        result
    }

    /// Compute the result of movement on one selection region.
    pub fn moved_selection_region
    (&self, movement:Movement, region:Selection, modify:bool) -> Selection {
        let text        = self.text();
        let no_horiz    = |t|(t,None);
        let (end,horiz) = match movement {

            Up                => self.vertical_motion(region, -1, modify),
            Down              => self.vertical_motion(region,  1, modify),
            UpExactPosition   => self.vertical_motion_exact_pos(region, true, modify),
            DownExactPosition => self.vertical_motion_exact_pos(region, false, modify),
            UpPage            => self.vertical_motion(region, -self.page_scroll_height(), modify),
            DownPage          => self.vertical_motion(region,  self.page_scroll_height(), modify),
            StartOfDocument   => no_horiz(0),
            EndOfDocument     => no_horiz(text.len()),

            Movement::Left => {
                let def     = (0,region.horiz);
                let do_move = region.is_caret() || modify;
                if  do_move { text.prev_grapheme_offset(region.end).map(no_horiz).unwrap_or(def) }
                else        { no_horiz(region.min()) }
            }

            Movement::Right => {
                let def     = (region.end,region.horiz);
                let do_move = region.is_caret() || modify;
                if  do_move { text.next_grapheme_offset(region.end).map(no_horiz).unwrap_or(def) }
                else        { no_horiz(region.max()) }
            }

            Movement::LeftOfLine => {
                let line   = self.line_of_offset(region.end);
                let offset = self.offset_of_line(line);
                no_horiz(offset)
            }

            Movement::RightOfLine => {
                let line             = self.line_of_offset(region.end);
                let text_len         = text.len();
                let last_line        = line == self.line_of_offset(text_len);
                let next_line_offset = self.offset_of_line(line+1);
                let offset           = if last_line { text_len } else {
                    text.prev_grapheme_offset(next_line_offset).unwrap_or(text_len)
                };
                no_horiz(offset)
            }

            Movement::StartOfParagraph => {
                // Note: TextEdit would start at modify ? region.end : region.min()
                let mut cursor = rope::Cursor::new(&text,region.end);
                let offset     = cursor.prev::<LinesMetric>().unwrap_or(0);
                no_horiz(offset)
            }

            Movement::EndOfParagraph => {
                // Note: TextEdit would start at modify ? region.end : region.max()
                let mut cursor = rope::Cursor::new(&text,region.end);
                let     offset = match cursor.next::<LinesMetric>() {
                    None            => text.len(),
                    Some(next_line) => {
                        if cursor.is_boundary::<LinesMetric>() {
                            text.prev_grapheme_offset(next_line).unwrap_or(region.end)
                        } else if cursor.pos() == text.len() {
                            text.len()
                        } else {
                            region.end
                        }
                    }
                };
                no_horiz(offset)
            }

            Movement::EndOfParagraphKill => {
                // Note: TextEdit would start at modify ? region.end : region.max()
                let mut cursor = rope::Cursor::new(&text,region.end);
                let     offset = match cursor.next::<LinesMetric>() {
                    None            => region.end,
                    Some(next_line) => {
                        if cursor.is_boundary::<LinesMetric>() {
                            let eol = text.prev_grapheme_offset(next_line);
                            eol.and_then(|t|(t!=region.end).as_some(t)).unwrap_or(next_line)
                        } else { next_line }
                    }
                };
                no_horiz(offset)
            }
        };
        let start = if modify { region.start } else { end };
        Selection::new(start,end).with_horiz(horiz)
    }
}


/// A set of zero or more selection regions, representing a selection state.
#[derive(Default, Debug, Clone)]
pub struct SelectionGroup {
    // An invariant: regions[i].max() <= regions[i+1].min()
    // and < if either is_caret()
    regions: Vec<Selection>,
}

/// Implementing the Deref trait allows callers to easily test `is_empty`, iterate
/// through all ranges, etc.
impl Deref for SelectionGroup {
    type Target = [Selection];
    fn deref(&self) -> &[Selection] {
        &self.regions
    }
}

impl SelectionGroup {
    /// Creates a new empty selection.
    pub fn new() -> SelectionGroup {
        SelectionGroup::default()
    }

    /// Add a region to the selection. This method implements merging logic.
    ///
    /// Two non-caret regions merge if their interiors intersect; merely
    /// touching at the edges does not cause a merge. A caret merges with
    /// a non-caret if it is in the interior or on either edge. Two carets
    /// merge if they are the same offset.
    ///
    /// Performance note: should be O(1) if the new region strictly comes
    /// after all the others in the selection, otherwise O(n).
    pub fn add_region(&mut self, region: Selection) {
        let mut ix = self.search(region.min());
        if ix == self.regions.len() {
            self.regions.push(region);
            return;
        }
        let mut region = region;
        let mut end_ix = ix;
        if self.regions[ix].min() <= region.min() {
            if self.regions[ix].should_merge(region) {
                region = region.merge_with(self.regions[ix]);
            } else {
                ix += 1;
            }
            end_ix += 1;
        }
        while end_ix < self.regions.len() && region.should_merge(self.regions[end_ix]) {
            region = region.merge_with(self.regions[end_ix]);
            end_ix += 1;
        }
        if ix == end_ix {
            self.regions.insert(ix, region);
        } else {
            self.regions[ix] = region;
            remove_n_at(&mut self.regions, ix + 1, end_ix - ix - 1);
        }
    }


    // The smallest index so that offset > region.max() for all preceding
    // regions.
    pub fn search(&self, offset: usize) -> usize {
        if self.regions.is_empty() || offset > self.regions.last().unwrap().max() {
            return self.regions.len();
        }
        match self.regions.binary_search_by(|r| r.max().cmp(&offset)) {
            Ok(ix) => ix,
            Err(ix) => ix,
        }
    }
}


pub fn remove_n_at<T: Clone>(v: &mut Vec<T>, index: usize, n: usize) {
    match n.cmp(&1) {
        std::cmp::Ordering::Equal => {
            v.remove(index);
        }
        std::cmp::Ordering::Greater => {
            let new_len = v.len() - n;
            for i in index..new_len {
                v[i] = v[i + n].clone();
            }
            v.truncate(new_len);
        }
        std::cmp::Ordering::Less => (),
    }
}






/// The specification of a movement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Movement {
    /// Move to the left by one grapheme cluster.
    Left,
    /// Move to the right by one grapheme cluster.
    Right,
//    /// Move to the left by one word.
//    LeftWord,
//    /// Move to the right by one word.
//    RightWord,
    /// Move to left end of visible line.
    LeftOfLine,
    /// Move to right end of visible line.
    RightOfLine,
    /// Move up one visible line.
    Up,
    /// Move down one visible line.
    Down,
    /// Move up one viewport height.
    UpPage,
    /// Move down one viewport height.
    DownPage,
    /// Move up to the next line that can preserve the cursor position.
    UpExactPosition,
    /// Move down to the next line that can preserve the cursor position.
    DownExactPosition,
    /// Move to the start of the text line.
    StartOfParagraph,
    /// Move to the end of the text line.
    EndOfParagraph,
    /// Move to the end of the text line, or next line if already at end.
    EndOfParagraphKill,
    /// Move to the start of the document.
    StartOfDocument,
    /// Move to the end of the document
    EndOfDocument,
}


// ==================
// === LineOffset ===
// ==================

/// A trait from which lines and columns in a document can be calculated
/// into offsets inside a rope an vice versa.
pub trait LineOffset {
    // use own breaks if present, or text if not (no line wrapping)

    fn text(&self) -> &Rope;

    /// Returns the byte offset corresponding to the given line.
    fn offset_of_line(&self, line:usize) -> usize {
        self.text().offset_of_line(line)
    }

    /// Returns the visible line number containing the given offset.
    fn line_of_offset(&self, offset:usize) -> usize {
        self.text().line_of_offset(offset)
    }

    // How should we count "column"? Valid choices include:
    // * Unicode codepoints
    // * grapheme clusters
    // * Unicode width (so CJK counts as 2)
    // * Actual measurement in text layout
    // * Code units in some encoding
    //
    // Of course, all these are identical for ASCII. For now we use UTF-8 code units
    // for simplicity.

    fn offset_to_line_col(&self, offset:usize) -> (usize,usize) {
        let line = self.line_of_offset(offset);
        let col  = offset - self.offset_of_line(line);
        (line,col)
    }

    fn line_col_to_offset(&self, line:usize, col:location::Column) -> usize {
        let mut offset = self.offset_of_line(line).saturating_add(col.raw);
        if offset >= self.text().len() {
            offset = self.text().len();
            if self.line_of_offset(offset) <= line {
                return offset;
            }
        } else {
            // Snap to grapheme cluster boundary
            offset = self.text().prev_grapheme_offset(offset + 1).unwrap();
        }

        // clamp to end of line
        let next_line_offset = self.offset_of_line(line + 1);
        if offset >= next_line_offset {
            if let Some(prev) = self.text().prev_grapheme_offset(next_line_offset) {
                offset = prev;
            }
        }
        offset
    }

//    /// Get the line range of a selected region.
//    fn get_line_range(&self, text: &Rope, region: &Selection) -> Range<usize> {
//        let (first_line, _) = self.offset_to_line_col(text, region.min());
//        let (mut last_line, last_col) = self.offset_to_line_col(text, region.max());
//        if last_col == 0 && last_line > first_line {
//            last_line -= 1;
//        }
//
//        first_line..(last_line + 1)
//    }
}








use crate::prelude::*;
use ensogl::data::color;
use crate::display::shape::text::glyph::font;
use crate::display::shape::text::glyph::pen::PenIterator;
use glyph::Glyph;


// ============
// === Line ===
// ============

/// A structure keeping line of glyphs with proper alignment.
///
/// Not all the glyphs in `glyphs` vector may be actually in use. This structure is meant to keep
/// changing text, and for best performance it re-uses the created Glyphs (what means the specific
/// buffer space). Therefore you can set a cap for line length by using the `set_fixed_capacity`
/// method.
#[derive(Clone,CloneRef,Debug)]
pub struct Line {
    display_object : display::object::Instance,
    glyph_system   : glyph::System,
    content        : Rc<RefCell<String>>,
    glyphs         : Rc<RefCell<Vec<Glyph>>>,
    font_color     : Rc<Cell<color::Rgba>>,
    font_size      : Rc<Cell<f32>>,
    fixed_capacity : Rc<Cell<Option<usize>>>,
}

impl Line {
    /// Constructor.
    pub fn new(logger:impl AnyLogger, glyph_system:&glyph::System) -> Self {
        let logger         = Logger::sub(logger,"line");
        let display_object = display::object::Instance::new(logger);
        let glyph_system   = glyph_system.clone_ref();
        let font_size      = Rc::new(Cell::new(11.0));
        let font_color     = Rc::new(Cell::new(color::Rgba::new(0.0,0.0,0.0,1.0)));
        let content        = default();
        let glyphs         = default();
        let fixed_capacity = default();
        Line {display_object,glyph_system,glyphs,font_size,font_color,content,fixed_capacity}
    }

    /// Replace currently visible text.
    pub fn set_text<S:Into<String>>(&self, content:S) {
        *self.content.borrow_mut() = content.into();
        self.redraw();
    }
}


// === Setters ===

#[allow(missing_docs)]
impl Line {
    pub fn set_font_color<C:Into<color::Rgba>>(&self, color:C) {
        let color = color.into();
        self.font_color.set(color);
        for glyph in &*self.glyphs.borrow() {
            glyph.set_color(color);
        }
    }

    pub fn set_font_size(&self, size:f32) {
        self.font_size.set(size);
        self.redraw();
    }

    pub fn change_fixed_capacity(&self, count:Option<usize>) {
        self.fixed_capacity.set(count);
        self.resize();
    }

    pub fn set_fixed_capacity(&self, count:usize) {
        self.change_fixed_capacity(Some(count));
    }

    pub fn unset_fixed_capacity(&self) {
        self.change_fixed_capacity(None);
    }
}


// === Getters ===

#[allow(missing_docs)]
impl Line {
    pub fn font_size(&self) -> f32 {
        self.font_size.get()
    }

    pub fn length(&self) -> usize {
        self.content.borrow().len()
    }

//    pub fn font(&self) -> font::Handle {
//        self.glyph_system.font.clone_ref()
//    }
}


// === Internal API ===

impl Line {
    /// Resizes the line to contain enough glyphs to display the full `content`. In case the
    /// `fixed_capacity` was set, it will add or remove the glyphs to match it.
    fn resize(&self) {
        let content_len        = self.content.borrow().len();
        let target_glyph_count = self.fixed_capacity.get().unwrap_or(content_len);
        let glyph_count        = self.glyphs.borrow().len();
        if target_glyph_count > glyph_count {
            let new_count  = target_glyph_count - glyph_count;
            let new_glyphs = (0..new_count).map(|_| {
                let glyph = self.glyph_system.new_glyph();
                self.add_child(&glyph);
                glyph
            });
            self.glyphs.borrow_mut().extend(new_glyphs)
        }
        if glyph_count > target_glyph_count {
            self.glyphs.borrow_mut().truncate(target_glyph_count)
        }
    }

    /// Updates properties of all glyphs, including characters they display, size, and colors.
    fn redraw(&self) {
        self.resize();

        let content     = self.content.borrow();
        let font        = self.glyph_system.font.clone_ref();
        let font_size   = self.font_size.get();
        let chars       = content.chars();
        let pen         = PenIterator::new(font_size,chars,font);
        let content_len = content.len();
        let color       = self.font_color.get();

        for (glyph,(chr,x_offset)) in self.glyphs.borrow().iter().zip(pen) {
            let glyph_info   = self.glyph_system.font.get_glyph_info(chr);
            let size         = glyph_info.scale.scale(font_size);
            let glyph_offset = glyph_info.offset.scale(font_size);
            let glyph_x      = x_offset + glyph_offset.x;
            let glyph_y      = glyph_offset.y;
            glyph.set_position(Vector3::new(glyph_x,glyph_y,0.0));
            glyph.set_glyph(chr);
            glyph.set_color(color);
            glyph.size.set(size);
        }

        for glyph in self.glyphs.borrow().iter().skip(content_len) {
            glyph.size.set(Vector2::new(0.0,0.0));
        }
    }
}


// === Display Object ===

impl display::Object for Line {
    fn display_object(&self) -> &display::object::Instance {
        &self.display_object
    }
}




///// Test.
//pub fn main() {
////    let mut text = Rope::from("hello\nworld\n!!!\nyo");
////    let mut cursor = Cursor::new(&text, 0);
////
////    while cursor.pos() < text.len() - 2 {
////        cursor.next::<BaseMetric>();
////
////        println!("{:?}",cursor.pos());
////    }
////    a.edit(5..6, "!");
////    for i in 0..1000000 {
////        let l = a.len();
////        a.edit(l..l, &(i.to_string() + "\n"));
////    }
////    let l = a.len();
////    for s in a.clone().iter_chunks(1000..3000) {
////        println!("chunk {:?}", s);
////    }
////    a.edit(1000..l, "");
////    //a = a.subrange(0, 1000);
////    println!("{:?}", String::from(a));
//}