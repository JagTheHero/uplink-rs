//! Port of lib/vanbakel/interface.cpp + interface.h (the CPU-usage task bar UI)
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]

use uplink_engine::{gl, text};

use crate::ctx::Ctx;
use crate::eclipse::button::Button;
use crate::interface::taskmanager::uplinktask::UplinkTask;
use crate::vanbakel::Vanbakel;

/// An RGBA colour as passed to `glColor4f`.
type Rgba = [f32; 4];

/// The `if (clicked) .. else if (highlighted) .. else ..` colour ladder used by `Svb_button_draw`.
fn state_colour(clicked: bool, highlighted: bool, on_click: Rgba, on_highlight: Rgba, normal: Rgba) -> Rgba {
    if clicked {
        on_click
    } else if highlighted {
        on_highlight
    } else {
        normal
    }
}

/// `sscanf(button->name, "%s %d", unused, &index)`: the number after the first word of a
/// button name such as "svb_priority 3". PORT: the C++ left `index` uninitialised when the
/// scan failed; 0 is used instead (the names are generated here, so this never happens).
fn button_index(name: &str) -> i32 {
    name.split_whitespace().nth(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0)
}

// PORT: the 6-argument C++ `EclRegisterButton(x, y, w, h, caption, name)` leaves the button's
// tooltip as " " ("no tooltip"; see `tooltip_update`), so those calls pass " " here, not "".
// Only `svb_titlebar` (a 7-argument call in the C++) really has an empty tooltip.

/// `Svb_initialise_interface`: call before anything else.
pub fn initialise_interface(g: &mut Ctx) {}

/// `SvbCreateInterface(x, y)` (defined in vanbakel.cpp): stores `g.svb.xpos/ypos`, creates a
/// column per task and each task's interface, sets `numregisteredbuttons`.
pub fn create_interface(g: &mut Ctx, x: i32, y: i32) {
    g.svb.xpos = x;
    g.svb.ypos = y;

    if !Vanbakel::is_interface_visible(g) {
        // Create a button which covers the scale bar (to ensure it is properly masked when removed)
        g.ecl.register_button(x - 3, y - 4, 106, 11, "", " ", "svb_mask");
        g.ecl.register_button_callbacks("svb_mask", None, None, None, None);

        // Create a title bar
        g.ecl.register_button(x, y - 15, 100, 12, "CPU usage", "", "svb_titlebar");
        g.ecl.register_button_callbacks("svb_titlebar", Some(textbutton_draw), None, None, None);

        // Create the scale bar
        g.ecl.register_button(x, y, 100, 5, "0...........50...........100", " ", "svb_scalebar");
        g.ecl.register_button_callbacks("svb_scalebar", Some(button_draw), None, None, None);

        // Create the task buttons
        let mut i = 0;
        while i < g.svb.num_tasks() {
            let name = g.svb.tasklist[i as usize].name.clone();
            create_column(g, x, y, &name, i);
            let pid = g.svb.tasklist[i as usize].pid;
            Vanbakel::with_task(g, pid, |t, g| t.create_interface(g));
            i += 1;
        }

        g.svb.numregisteredbuttons = g.svb.num_tasks();
    }
}

/// `SvbRemoveInterface` (defined in vanbakel.cpp)
pub fn remove_interface(g: &mut Ctx) {
    if Vanbakel::is_interface_visible(g) {
        // Remove the scale bar
        g.ecl.remove_button("svb_mask");
        g.ecl.remove_button("svb_titlebar");
        g.ecl.remove_button("svb_scalebar");

        // Remove the task buttons
        let mut i = 0;
        while i < g.svb.num_tasks() {
            remove_column(g, i);
            let pid = g.svb.tasklist[i as usize].pid;
            Vanbakel::with_task(g, pid, |t, g| t.remove_interface(g));
            i += 1;
        }
    }
}

/// `SvbUpdateInterface` (defined in vanbakel.cpp): adds/removes columns to match the task count.
pub fn update_interface(g: &mut Ctx) {
    if Vanbakel::is_interface_visible(g) {
        let (xpos, ypos) = (g.svb.xpos, g.svb.ypos);

        // Add / remove task buttons as neccisary
        let numtasks = g.svb.num_tasks();
        let registered = g.svb.numregisteredbuttons;
        if numtasks > registered {
            // Add more buttons
            for i in registered..numtasks {
                let name = g.svb.tasklist[i as usize].name.clone();
                create_column(g, xpos, ypos, &name, i);
            }
        } else if numtasks < registered {
            // Remove some buttons
            for i in numtasks..registered {
                remove_column(g, i);
            }
        }

        g.svb.numregisteredbuttons = g.svb.num_tasks();

        // Resize/update the remaining buttons
        for i in 0..g.svb.num_tasks() {
            let (priority, name) = {
                let tw = &g.svb.tasklist[i as usize];
                (tw.priority, tw.name.clone())
            };

            let bname = format!("svb_priority {i}");
            g.ecl.register_resize(&bname, (priority * 100.0) as i32, 10, 500, None);
            if let Some(button) = g.ecl.get_button_mut(&bname) {
                button.set_caption(&name);
            }
        }
    }
}

/// `Svb_button_draw`: draws a button fairly normally, text is left justified.
pub fn button_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let xpos = button.x + 2;
    let ypos = (button.y + button.height / 2) + 1;

    gl::begin(gl::QUADS);

    let c = state_colour(clicked, highlighted, [0.5, 0.5, 1.0, 0.6], [0.2, 0.2, 0.9, 0.6], [0.0, 0.0, 0.4, 0.6]);
    gl::color4f(c[0], c[1], c[2], c[3]);
    gl::vertex3i(button.x, button.y, 0);

    let c = state_colour(clicked, highlighted, [0.7, 0.7, 1.0, 0.6], [0.5, 0.5, 1.0, 0.6], [0.0, 0.0, 0.7, 0.6]);
    gl::color4f(c[0], c[1], c[2], c[3]);
    gl::vertex3i(button.x + button.width, button.y, 0);

    let c = state_colour(clicked, highlighted, [0.5, 0.5, 1.0, 0.6], [0.2, 0.2, 0.9, 0.6], [0.0, 0.0, 0.4, 0.6]);
    gl::color4f(c[0], c[1], c[2], c[3]);
    gl::vertex3i(button.x + button.width, button.y + button.height, 0);

    let c = state_colour(clicked, highlighted, [0.7, 0.7, 1.0, 0.6], [0.5, 0.5, 1.0, 0.6], [0.0, 0.0, 0.7, 0.6]);
    gl::color4f(c[0], c[1], c[2], c[3]);
    gl::vertex3i(button.x, button.y + button.height, 0);

    gl::end();

    gl::color4f(1.0, 1.0, 1.0, 0.8);
    text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_10);
}

/// `Svb_textbutton_draw`: draws a button with text only - no background.
pub fn textbutton_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    // Clear the background
    // PORT: the C++ emitted this black quad's four glVertex2i calls with no glBegin(GL_QUADS)
    // (and a bare glEnd). OpenGL discards vertices and glEnd outside a primitive, so the
    // background was never drawn; the vertices are not emitted here to keep that behaviour.
    gl::color4f(0.0, 0.0, 0.0, 1.0);

    // Draw the text
    let xpos = button.x + 2;
    let ypos = (button.y + button.height / 2) + 1;

    gl::color4f(1.0, 1.0, 1.0, 0.8);
    text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_10);
}

/// `Svb_column_draw`: draws a coloured column, colour is determined by length.
pub fn column_draw(g: &mut Ctx, button: &Button, highlighted: bool, clicked: bool) {
    let xpos = button.x + 5;
    let ypos = (button.y + button.height / 2) + 2;

    // C++: (float) button->width / 50.0 is evaluated in double and stored as float.
    let mut scale = (button.width as f64 / 50.0) as f32;
    if highlighted {
        scale *= 2.0;
    }

    gl::begin(gl::QUADS);

    gl::color4f(0.0, 2.0 - scale, scale, 0.6);
    gl::vertex3i(button.x, button.y, 0);

    gl::color4f(0.0, 2.0 - scale, scale, 0.6);
    gl::vertex3i(button.x + button.width, button.y, 0);

    gl::color4f(0.0, 2.0 - scale, scale, 0.6);
    gl::vertex3i(button.x + button.width, button.y + button.height, 0);

    gl::color4f(0.0, 2.0 - scale, scale, 0.6);
    gl::vertex3i(button.x, button.y + button.height, 0);

    gl::end();

    gl::color4f(1.0, 1.0, 1.0, 1.0);
    text::draw_text_styled(xpos, ypos, &button.caption, text::HELVETICA_10);
}

/// `Svb_button_highlight` (mouse-move callback)
pub fn button_highlight(g: &mut Ctx, button: &Button) {
    g.ecl.highlight_button(&button.name);
    // PORT: C++ GciPostRedisplay() is not needed; the frame loop redraws every frame.
}

/// `Svb_button_click` (mouse-down callback)
pub fn button_click(g: &mut Ctx, button: &Button) {
    g.ecl.click_button(&button.name);
    // PORT: C++ GciPostRedisplay() is not needed; the frame loop redraws every frame.
}

/// `Svb_decrease_click`: button name "svb_decrease N".
pub fn decrease_click(g: &mut Ctx, button: &Button) {
    let index = button_index(&button.name);

    g.svb.change_priority(index, -0.2, false);
    g.svb.compensate_priorities();

    Vanbakel::update_interface(g);
    // PORT: C++ GciPostRedisplay() is not needed; the frame loop redraws every frame.
}

/// `Svb_column_click`: button name "svb_priority N"; shows the task at index N.
pub fn column_click(g: &mut Ctx, button: &Button) {
    let index = button_index(&button.name);

    let Some(tw) = g.svb.get_task_wrapper_at_index(index) else {
        debug_assert!(false, "Svb_column_click: no task at index {index}");
        return;
    };
    let pid = tw.pid;

    Vanbakel::show_task(g, pid);
}

/// `Svb_increase_click`: button name "svb_increase N".
pub fn increase_click(g: &mut Ctx, button: &Button) {
    let index = button_index(&button.name);

    g.svb.change_priority(index, 0.2, false);
    g.svb.compensate_priorities();

    Vanbakel::update_interface(g);
    // PORT: C++ GciPostRedisplay() is not needed; the frame loop redraws every frame.
}

/// `SvbCreateColumn(baseX, baseY, tw, taskindex)`; PORT: takes the task's name (the only
/// field of the `TaskWrapper` it used) instead of the wrapper pointer.
pub fn create_column(g: &mut Ctx, base_x: i32, base_y: i32, task_name: &str, taskindex: i32) {
    let row_y = base_y + 8 + taskindex * 15;

    // Create a mask button to completely cover the bar
    let name = format!("svb_priority_mask {taskindex}");
    g.ecl.register_button(base_x, row_y, 100, 10, "", " ", &name);
    g.ecl.register_button_callbacks(&name, None, None, None, None);

    // Create the bar
    let name = format!("svb_priority {taskindex}");
    g.ecl.register_button(base_x, row_y, 0, 10, task_name, " ", &name);
    g.ecl.register_button_callbacks(
        &name,
        Some(column_draw),
        Some(column_click),
        Some(button_click),
        Some(button_highlight),
    );

    // Create the priority change buttons
    let name_decrease = format!("svb_decrease {taskindex}");
    let name_increase = format!("svb_increase {taskindex}");

    g.ecl.register_button(base_x - 11, row_y, 10, 10, "<", " ", &name_decrease);
    g.ecl.register_button(base_x + 101, row_y, 10, 10, ">", " ", &name_increase);

    g.ecl.register_button_callbacks(
        &name_decrease,
        Some(button_draw),
        Some(decrease_click),
        Some(button_click),
        Some(button_highlight),
    );
    g.ecl.register_button_callbacks(
        &name_increase,
        Some(button_draw),
        Some(increase_click),
        Some(button_click),
        Some(button_highlight),
    );
}

/// `SvbRemoveColumn(taskindex)`
pub fn remove_column(g: &mut Ctx, taskindex: i32) {
    g.ecl.remove_button(&format!("svb_priority {taskindex}"));
    g.ecl.remove_button(&format!("svb_decrease {taskindex}"));
    g.ecl.remove_button(&format!("svb_increase {taskindex}"));
    g.ecl.remove_button(&format!("svb_priority_mask {taskindex}"));
}
