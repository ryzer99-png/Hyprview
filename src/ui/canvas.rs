use crate::models::geometry::LogicalRect;
use crate::models::Monitor;
use gtk4::cairo;
use gtk4::pango;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
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

            if let Some(name) = Self::hit_test(&s, x, y, w, h) {
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

            if let Some(name) = Self::hit_test(&s, x, y, w, h) {
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

        // Add 300px logical padding
        let padding = 200.0;
        let total_w = (max_x - min_x + padding * 2.0).max(1000.0);
        let total_h = (max_y - min_y + padding * 2.0).max(800.0);

        let scale_x = canvas_w / total_w;
        let scale_y = canvas_h / total_h;
        let scale = scale_x.min(scale_y).clamp(0.05, 0.45);

        // Center in canvas
        let bounding_center_x = (min_x + max_x) / 2.0;
        let bounding_center_y = (min_y + max_y) / 2.0;

        let canvas_center_x = canvas_w / 2.0;
        let canvas_center_y = canvas_h / 2.0;

        let origin_x = canvas_center_x - bounding_center_x * scale;
        let origin_y = canvas_center_y - bounding_center_y * scale;

        (scale, origin_x, origin_y)
    }

    fn hit_test(
        state: &DisplayCanvasState,
        click_x: f64,
        click_y: f64,
        canvas_w: f64,
        canvas_h: f64,
    ) -> Option<String> {
        let (scale, origin_x, origin_y) = Self::get_transform(state, canvas_w, canvas_h);

        // Reverse search so topmost monitor gets hit first
        for m in state.monitors.iter().rev() {
            let (w, h) = m.logical_size();
            let rect_x = origin_x + m.position.x as f64 * scale;
            let rect_y = origin_y + m.position.y as f64 * scale;
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
        // Background: modern dark slate canvas
        cr.set_source_rgb(0.12, 0.13, 0.15); // #1f2126
        let _ = cr.paint();

        // Draw subtle dot grid
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.05);
        let grid_step = 24.0;
        let mut x = 0.0;
        while x < width {
            let mut y = 0.0;
            while y < height {
                cr.arc(x, y, 1.2, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
                y += grid_step;
            }
            x += grid_step;
        }

        if state.monitors.is_empty() {
            return;
        }

        let (scale, origin_x, origin_y) = Self::get_transform(state, width, height);

        for m in &state.monitors {
            let is_selected = state.selected_name.as_deref() == Some(&m.name);
            let (log_w, log_h) = m.logical_size();

            let rx = origin_x + m.position.x as f64 * scale;
            let ry = origin_y + m.position.y as f64 * scale;
            let rw = log_w * scale;
            let rh = log_h * scale;

            // Draw monitor card background
            if m.enabled {
                if is_selected {
                    cr.set_source_rgb(0.22, 0.25, 0.32); // active card
                } else {
                    cr.set_source_rgb(0.18, 0.19, 0.23); // normal card
                }
            } else {
                cr.set_source_rgba(0.16, 0.16, 0.18, 0.6); // disabled card
            }

            draw_rounded_rect(cr, rx, ry, rw, rh, 10.0);
            let _ = cr.fill();

            // Border
            if is_selected {
                cr.set_source_rgb(0.21, 0.51, 0.96); // Libadwaita accent blue #3584e4
                cr.set_line_width(2.5);
            } else if m.enabled {
                cr.set_source_rgb(0.30, 0.32, 0.38);
                cr.set_line_width(1.2);
            } else {
                cr.set_source_rgba(0.3, 0.3, 0.3, 0.4);
                cr.set_line_width(1.0);
            }
            draw_rounded_rect(cr, rx, ry, rw, rh, 10.0);
            let _ = cr.stroke();

            // Monitor labels using Pango
            let pango_ctx = area.pango_context();
            let layout = pango::Layout::new(&pango_ctx);

            let label_markup = if m.enabled {
                format!(
                    "<span font_desc='Sans Bold 11' color='white'>{}</span>\n<span font_desc='Sans 9' color='#9aa0a6'>{} ({:.0}%)</span>",
                    m.name,
                    m.active_mode,
                    m.scale * 100.0
                )
            } else {
                format!(
                    "<span font_desc='Sans Bold 11' color='#808080'>{}</span>\n<span font_desc='Sans 9' color='#606060'>[Disabled]</span>",
                    m.name
                )
            };

            layout.set_markup(&label_markup);
            layout.set_alignment(pango::Alignment::Center);

            let (_, extents) = layout.pixel_extents();
            let text_x = rx + (rw - extents.width() as f64) / 2.0;
            let text_y = ry + (rh - extents.height() as f64) / 2.0;

            cr.move_to(text_x, text_y);
            pangocairo::functions::show_layout(cr, &layout);

            // Badge for Internal/Primary
            if m.is_internal() {
                let badge_layout = pango::Layout::new(&pango_ctx);
                badge_layout.set_markup("<span font_desc='Sans Bold 7' color='#3584e4'>LAPTOP</span>");
                cr.move_to(rx + 8.0, ry + 6.0);
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
