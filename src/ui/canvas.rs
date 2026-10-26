use crate::models::geometry::LogicalRect;
use crate::models::Monitor;
use gtk4::cairo;
use gtk4::pango;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct DragState {
    pub monitor_name: String,
    pub original_x: i32,
    pub original_y: i32,
    pub start_mouse_x: f64,
    pub start_mouse_y: f64,
}

pub struct DisplayCanvasState {
    pub monitors: Vec<Monitor>,
    pub selected_name: Option<String>,
    pub drag: Option<DragState>,
}

#[derive(Clone)]
pub struct DisplayCanvas {
    pub widget: gtk4::DrawingArea,
    state: Rc<RefCell<DisplayCanvasState>>,
}

impl DisplayCanvas {
    pub fn new<FSelect, FMove>(on_select: FSelect, on_move: FMove) -> Self
    where
        FSelect: Fn(String) + 'static,
        FMove: Fn(String, i32, i32) + 'static,
    {
        let widget = gtk4::DrawingArea::builder()
            .hexpand(true)
            .vexpand(true)
            .focusable(true)
            .can_target(true)
            .build();

        let state = Rc::new(RefCell::new(DisplayCanvasState {
            monitors: Vec::new(),
            selected_name: None,
            drag: None,
        }));

        // Setup draw function
        let draw_state = Rc::clone(&state);
        widget.set_draw_func(move |area, cr, width, height| {
            let s = draw_state.borrow();
            Self::draw(&s, area, cr, width as f64, height as f64);
        });

        // Setup click gesture
        let click_state = Rc::clone(&state);
        let click_widget = widget.clone();
        let on_select = Rc::new(on_select);
        let on_select_clone = Rc::clone(&on_select);

        let click_gesture = gtk4::GestureClick::new();
        click_gesture.connect_pressed(move |_, _, x, y| {
            let mut s = click_state.borrow_mut();
            let w = click_widget.width() as f64;
            let h = click_widget.height() as f64;

            if let Some(name) = Self::hit_test(&s, &click_widget, x, y, w, h) {
                s.selected_name = Some(name.clone());
                click_widget.queue_draw();
                on_select_clone(name);
            }
        });
        widget.add_controller(click_gesture);

        // Setup drag gesture
        let drag_state = Rc::clone(&state);
        let drag_widget = widget.clone();
        let on_move = Rc::new(on_move);

        let drag_gesture = gtk4::GestureDrag::new();
        let drag_state_start = Rc::clone(&drag_state);
        let drag_widget_start = drag_widget.clone();

        drag_gesture.connect_drag_begin(move |_, x, y| {
            let mut s = drag_state_start.borrow_mut();
            let w = drag_widget_start.width() as f64;
            let h = drag_widget_start.height() as f64;

            // Don't drag if clicked on the top pills
            if y <= 48.0 {
                return;
            }

            if let Some(name) = Self::hit_test(&s, &drag_widget_start, x, y, w, h) {
                let mon_pos = s
                    .monitors
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| (m.position.x, m.position.y));

                if let Some((pos_x, pos_y)) = mon_pos {
                    s.selected_name = Some(name.clone());
                    s.drag = Some(DragState {
                        monitor_name: name,
                        original_x: pos_x,
                        original_y: pos_y,
                        start_mouse_x: x,
                        start_mouse_y: y,
                    });
                    drag_widget_start.queue_draw();
                }
            }
        });

        let drag_state_update = Rc::clone(&drag_state);
        let drag_widget_update = drag_widget.clone();
        let on_move_update = Rc::clone(&on_move);

        drag_gesture.connect_drag_update(move |_, offset_x, offset_y| {
            let mut s = drag_state_update.borrow_mut();
            if let Some(drag) = s.drag.clone() {
                let w = drag_widget_update.width() as f64;
                let h = drag_widget_update.height() as f64;

                let (scale, _origin_x, _origin_y) = Self::get_transform(&s, w, h);
                let delta_logical_x = (offset_x / scale).round() as i32;
                let delta_logical_y = (offset_y / scale).round() as i32;

                let mut new_x = drag.original_x + delta_logical_x;
                let mut new_y = drag.original_y + delta_logical_y;

                // Compute magnetic snapping against other enabled monitors
                if let Some(mon_idx) = s.monitors.iter().position(|m| m.name == drag.monitor_name) {
                    let (mon_w, mon_h) = s.monitors[mon_idx].logical_size();
                    let mut candidate_rect = LogicalRect::new(new_x, new_y, mon_w, mon_h);

                    for (i, other) in s.monitors.iter().enumerate() {
                        if i != mon_idx && other.enabled {
                            let other_rect = other.logical_rect();
                            candidate_rect.snap_to(&other_rect, 40.0);
                        }
                    }

                    new_x = candidate_rect.origin.x;
                    new_y = candidate_rect.origin.y;

                    s.monitors[mon_idx].position.x = new_x;
                    s.monitors[mon_idx].position.y = new_y;

                    drag_widget_update.queue_draw();
                    on_move_update(drag.monitor_name.clone(), new_x, new_y);
                }
            }
        });

        let drag_state_end = Rc::clone(&drag_state);
        let drag_widget_end = drag_widget.clone();
        drag_gesture.connect_drag_end(move |_, _, _| {
            let mut s = drag_state_end.borrow_mut();
            s.drag = None;
            drag_widget_end.queue_draw();
        });

        widget.add_controller(drag_gesture);

        Self { widget, state }
    }

    pub fn set_monitors(&self, monitors: Vec<Monitor>) {
        let mut s = self.state.borrow_mut();
        if s.selected_name.is_none() && !monitors.is_empty() {
            s.selected_name = Some(monitors[0].name.clone());
        }
        s.monitors = monitors;
        self.widget.queue_draw();
    }

    pub fn set_selected(&self, name: String) {
        let mut s = self.state.borrow_mut();
        s.selected_name = Some(name);
        self.widget.queue_draw();
    }

    fn get_transform(state: &DisplayCanvasState, canvas_w: f64, canvas_h: f64) -> (f64, f64, f64) {
        if state.monitors.is_empty() {
            return (0.1, 0.0, 0.0);
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for m in &state.monitors {
            let (w, h) = m.logical_size();
            let x = m.position.x as f64;
            let y = m.position.y as f64;

            if x < min_x { min_x = x; }
            if y < min_y { min_y = y; }
            if x + w > max_x { max_x = x + w; }
            if y + h > max_y { max_y = y + h; }
        }

        let padding = 200.0;
        let total_w = (max_x - min_x + padding * 2.0).max(1000.0);
        let total_h = (max_y - min_y + padding * 2.0).max(800.0);

        let scale_x = canvas_w / total_w;
        let scale_y = (canvas_h - 50.0) / total_h; // leave 50px for top pill bar
        let scale = scale_x.min(scale_y).clamp(0.05, 0.45);

        let bounding_center_x = (min_x + max_x) / 2.0;
        let bounding_center_y = (min_y + max_y) / 2.0;

        let canvas_center_x = canvas_w / 2.0;
        let canvas_center_y = (canvas_h + 30.0) / 2.0;

        let origin_x = canvas_center_x - bounding_center_x * scale;
        let origin_y = canvas_center_y - bounding_center_y * scale;

        (scale, origin_x, origin_y)
    }

    /// Computes layout rectangles for the top monitor quick-selection pills
    fn get_pill_rects(state: &DisplayCanvasState, area: &gtk4::DrawingArea) -> Vec<(String, f64, f64, f64, f64)> {
        let pango_ctx = area.pango_context();
        let layout = pango::Layout::new(&pango_ctx);

        let mut pills = Vec::new();
        let mut curr_x = 16.0;
        let pill_y = 12.0;
        let pill_h = 30.0;

        for m in &state.monitors {
            let label = if m.is_internal() {
                format!("💻 {}", m.name)
            } else {
                format!("🖥️ {}", m.name)
            };
            layout.set_text(&label);
            let (w, _) = layout.pixel_size();
            let pill_w = (w as f64 + 28.0).max(80.0);

            pills.push((m.name.clone(), curr_x, pill_y, pill_w, pill_h));
            curr_x += pill_w + 10.0;
        }

        pills
    }

    /// Calculates visual cascade offset when two displays share identical coordinates (e.g. mirrored)
    fn get_stack_offset(mon: &Monitor, state: &DisplayCanvasState) -> (f64, f64) {
        let same_pos: Vec<&Monitor> = state
            .monitors
            .iter()
            .filter(|other| other.position.x == mon.position.x && other.position.y == mon.position.y)
            .collect();

        if same_pos.len() <= 1 {
            return (0.0, 0.0);
        }

        // If this is the selected monitor, keep it front and centered or slightly elevated
        let is_selected = state.selected_name.as_deref() == Some(&mon.name);
        if is_selected {
            (0.0, 0.0)
        } else {
            // Find index of this unselected monitor in the stack
            let idx = same_pos
                .iter()
                .position(|m| m.name == mon.name)
                .unwrap_or(0);
            let offset = (idx as f64 + 1.0) * 16.0;
            (-offset, -offset)
        }
    }

    fn hit_test(
        state: &DisplayCanvasState,
        area: &gtk4::DrawingArea,
        click_x: f64,
        click_y: f64,
        canvas_w: f64,
        canvas_h: f64,
    ) -> Option<String> {
        // 1. Check top pill bar first
        let pills = Self::get_pill_rects(state, area);
        for (name, px, py, pw, ph) in pills {
            if click_x >= px && click_x <= px + pw && click_y >= py && click_y <= py + ph {
                return Some(name);
            }
        }

        // 2. Check cards in Z-order: check selected monitor first, then others
        let (scale, origin_x, origin_y) = Self::get_transform(state, canvas_w, canvas_h);

        let mut sorted_monitors: Vec<&Monitor> = state.monitors.iter().collect();
        // Sort so selected monitor is checked first
        sorted_monitors.sort_by(|a, b| {
            let a_selected = state.selected_name.as_deref() == Some(&a.name);
            let b_selected = state.selected_name.as_deref() == Some(&b.name);
            b_selected.cmp(&a_selected)
        });

        for m in sorted_monitors {
            let (w, h) = m.logical_size();
            let (stack_x, stack_y) = Self::get_stack_offset(m, state);

            let rect_x = origin_x + m.position.x as f64 * scale + stack_x;
            let rect_y = origin_y + m.position.y as f64 * scale + stack_y;
            let rect_w = w * scale;
            let rect_h = h * scale;

            if click_x >= rect_x
                && click_x <= rect_x + rect_w
                && click_y >= rect_y
                && click_y <= rect_y + rect_h
            {
                return Some(m.name.clone());
            }
        }

        None
    }

    fn draw(
        state: &DisplayCanvasState,
        area: &gtk4::DrawingArea,
        cr: &cairo::Context,
        width: f64,
        height: f64,
    ) {
        // Canvas background
        cr.set_source_rgb(0.11, 0.12, 0.14); // #1c1d21
        let _ = cr.paint();

        // Subtle dot grid
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.04);
        let grid_step = 24.0;
        let mut gx = 0.0;
        while gx < width {
            let mut gy = 0.0;
            while gy < height {
                cr.arc(gx, gy, 1.2, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
                gy += grid_step;
            }
            gx += grid_step;
        }

        if state.monitors.is_empty() {
            return;
        }

        // Draw Top Quick-Selection Pills
        let pills = Self::get_pill_rects(state, area);
        let pango_ctx = area.pango_context();
        let pill_layout = pango::Layout::new(&pango_ctx);

        for (name, px, py, pw, ph) in &pills {
            let is_sel = state.selected_name.as_deref() == Some(name);

            // Pill background
            if is_sel {
                cr.set_source_rgb(0.21, 0.51, 0.96); // #3584e4 accent blue
            } else {
                cr.set_source_rgba(0.18, 0.20, 0.24, 0.95);
            }
            draw_rounded_rect(cr, *px, *py, *pw, *ph, ph / 2.0);
            let _ = cr.fill();

            // Pill border
            if is_sel {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
                cr.set_line_width(1.5);
            } else {
                cr.set_source_rgba(0.35, 0.38, 0.45, 0.6);
                cr.set_line_width(1.0);
            }
            draw_rounded_rect(cr, *px, *py, *pw, *ph, ph / 2.0);
            let _ = cr.stroke();

            // Pill text
            let is_internal = state.monitors.iter().find(|m| &m.name == name).map_or(false, |m| m.is_internal());
            let icon = if is_internal { "💻" } else { "🖥️" };
            let pill_text = format!(
                "<span font_desc='Sans Bold 9' color='{}'>{} {}</span>",
                if is_sel { "#ffffff" } else { "#cdd6f4" },
                icon,
                name
            );
            pill_layout.set_markup(&pill_text);
            let (tw, th) = pill_layout.pixel_size();
            cr.move_to(px + (pw - tw as f64) / 2.0, py + (ph - th as f64) / 2.0);
            pangocairo::functions::show_layout(cr, &pill_layout);
        }

        // Prepare monitors for drawing:
        // Sort so unselected are drawn FIRST, and selected monitor is drawn LAST (on top)
        let (scale, origin_x, origin_y) = Self::get_transform(state, width, height);

        let mut sorted_monitors: Vec<&Monitor> = state.monitors.iter().collect();
        sorted_monitors.sort_by(|a, b| {
            let a_selected = state.selected_name.as_deref() == Some(&a.name);
            let b_selected = state.selected_name.as_deref() == Some(&b.name);
            a_selected.cmp(&b_selected)
        });

        for m in sorted_monitors {
            let is_selected = state.selected_name.as_deref() == Some(&m.name);
            let (log_w, log_h) = m.logical_size();
            let (stack_x, stack_y) = Self::get_stack_offset(m, state);

            let rx = origin_x + m.position.x as f64 * scale + stack_x;
            let ry = origin_y + m.position.y as f64 * scale + stack_y;
            let rw = log_w * scale;
            let rh = log_h * scale;

            // 1. Drop shadow (elevated shadow for selected card)
            if is_selected {
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.55);
                draw_rounded_rect(cr, rx + 4.0, ry + 8.0, rw, rh, 12.0);
                let _ = cr.fill();
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
                draw_rounded_rect(cr, rx + 2.0, ry + 4.0, rw, rh, 10.0);
                let _ = cr.fill();
            }

            // 2. Card background: translucent glassmorphism so overlapping cards are distinguishable
            if m.enabled {
                if is_selected {
                    cr.set_source_rgba(0.18, 0.22, 0.30, 0.94);
                } else if m.mirror_of.is_some() {
                    cr.set_source_rgba(0.14, 0.18, 0.24, 0.85); // mirrored card
                } else {
                    cr.set_source_rgba(0.15, 0.16, 0.20, 0.88); // normal card
                }
            } else {
                cr.set_source_rgba(0.12, 0.12, 0.14, 0.65); // disabled card
            }

            draw_rounded_rect(cr, rx, ry, rw, rh, 10.0);
            let _ = cr.fill();

            let is_overlapping = m.enabled && m.mirror_of.is_none() && state.monitors.iter().any(|other| {
                other.name != m.name && other.enabled && other.mirror_of.is_none() && m.logical_rect().overlaps(&other.logical_rect())
            });

            // 3. Border with high contrast
            if is_overlapping {
                cr.set_source_rgb(0.98, 0.70, 0.53); // Warning amber #fab387
                cr.set_line_width(2.6);
            } else if is_selected {
                cr.set_source_rgb(0.21, 0.51, 0.96); // Libadwaita accent blue #3584e4
                cr.set_line_width(2.8);
            } else if m.mirror_of.is_some() {
                cr.set_source_rgba(0.65, 0.89, 0.63, 0.8); // soft green border for mirrored
                cr.set_line_width(1.8);
            } else if m.enabled {
                cr.set_source_rgba(0.38, 0.40, 0.48, 0.8);
                cr.set_line_width(1.2);
            } else {
                cr.set_source_rgba(0.30, 0.30, 0.30, 0.4);
                cr.set_line_width(1.0);
            }
            draw_rounded_rect(cr, rx, ry, rw, rh, 10.0);
            let _ = cr.stroke();

            // 4. Labels
            let layout = pango::Layout::new(&pango_ctx);

            let label_markup = if !m.enabled {
                format!(
                    "<span font_desc='Sans Bold 11' color='#808080'>{}</span>\n<span font_desc='Sans 9' color='#606060'>[Disabled]</span>",
                    m.name
                )
            } else if let Some(ref target) = m.mirror_of {
                format!(
                    "<span font_desc='Sans Bold 11' color='white'>{}</span>\n<span font_desc='Sans 9' color='#a6e3a1'>🪞 Mirror of {}</span>",
                    m.name, target
                )
            } else {
                format!(
                    "<span font_desc='Sans Bold 11' color='white'>{}</span>\n<span font_desc='Sans 9' color='#9aa0a6'>{} ({:.0}%)</span>",
                    m.name,
                    m.active_mode,
                    m.scale * 100.0
                )
            };

            layout.set_markup(&label_markup);
            layout.set_alignment(pango::Alignment::Center);

            let (_, extents) = layout.pixel_extents();
            let text_x = rx + (rw - extents.width() as f64) / 2.0;
            let text_y = ry + (rh - extents.height() as f64) / 2.0;

            cr.move_to(text_x, text_y);
            pangocairo::functions::show_layout(cr, &layout);

            // 5. Badges
            let mut badge_offset_x = rx + 8.0;
            if m.is_internal() {
                let badge_layout = pango::Layout::new(&pango_ctx);
                badge_layout.set_markup("<span font_desc='Sans Bold 7' color='#3584e4'>LAPTOP</span>");
                cr.move_to(badge_offset_x, ry + 6.0);
                pangocairo::functions::show_layout(cr, &badge_layout);
                badge_offset_x += 48.0;
            }

            if m.mirror_of.is_some() {
                let badge_layout = pango::Layout::new(&pango_ctx);
                badge_layout.set_markup("<span font_desc='Sans Bold 7' color='#a6e3a1'>MIRROR</span>");
                cr.move_to(badge_offset_x, ry + 6.0);
                pangocairo::functions::show_layout(cr, &badge_layout);
                badge_offset_x += 52.0;
            }

            if is_overlapping {
                let badge_layout = pango::Layout::new(&pango_ctx);
                badge_layout.set_markup("<span font_desc='Sans Bold 7' color='#fab387'>⚠️ OVERLAP</span>");
                cr.move_to(badge_offset_x, ry + 6.0);
                pangocairo::functions::show_layout(cr, &badge_layout);
            }
        }
    }
}

fn draw_rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}
