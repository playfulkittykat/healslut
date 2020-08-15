// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::config::{Channel, Target, TargetSpec};
use crate::plug::{Capture, Plug};

use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};

use glib::MainContext;

use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Button, DialogBuilder, EventBox, FileChooserAction,
    FileChooserNative, Image, Orientation, Statusbar, Window, WindowBuilder,
    WindowType,
};

use std::cell::RefCell;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

thread_local! {
    static GUI: RefCell<Option<Rc<RefCell<Gui>>>> = RefCell::new(None);
}

#[derive(Debug)]
enum Event<'a> {
    StartScan,
    StopScan,
    Connect(&'a str),
    Disconnect,
}

#[derive(Debug)]
pub struct Gui {
    plug: Plug,
    scanning: bool,
    connected: bool,

    window: ApplicationWindow,
    top_box: gtk::Box,
    statusbar: Statusbar,
    btn_device: Button,
    btn_target: Button,

    status_connect: u32,
    status_scan: u32,
}

impl Gui {
    const SCAN: &'static str = "Scan for Toys";
    const STOP: &'static str = "Stop Scanning";
    const DISCONNECT: &'static str = "Disconnect";
    const TARGET: &'static str = "Set Target...";

    pub fn build(application: &gtk::Application, plug: Plug) {
        GUI.with(|cell| {
            let mut gui = cell.borrow_mut();

            if gui.is_some() {
                panic!("GUI already exists!");
            }

            let h_box = gtk::Box::new(Orientation::Horizontal, 10);
            let window = ApplicationWindow::new(application);
            let top_box = gtk::Box::new(Orientation::Vertical, 10);
            let btn_device = gtk::Button::with_label(Self::SCAN);
            let btn_target = gtk::Button::with_label(Self::TARGET);
            let statusbar = gtk::Statusbar::new();

            let status_scan = statusbar.get_context_id("scan");
            let status_connect = statusbar.get_context_id("connect");

            let rc = Rc::new(RefCell::new(Self {
                scanning: false,
                connected: false,
                plug,
                window,
                top_box,
                statusbar,
                btn_device,
                btn_target,
                status_connect,
                status_scan,
            }));

            {
                let new = rc.borrow_mut();

                let click_rc = Rc::downgrade(&rc);
                new.btn_device.connect_clicked(move |_| {
                    if let Some(up) = click_rc.upgrade() {
                        let gui = up.borrow_mut();

                        match (gui.scanning, gui.connected) {
                            (_, true) => gui.plug.disconnect(),
                            (false, false) => gui.plug.scan(),
                            (true, false) => gui.plug.stop_scan(),
                        }
                    }
                });

                let target_rc = Rc::downgrade(&rc);
                new.btn_target.connect_clicked(move |_| {
                    let chooser = match target_rc.upgrade() {
                        None => return,
                        Some(up) => {
                            let gui = up.borrow();

                            FileChooserNative::new(
                                Some("Choose a Screenshot"),
                                Some(&gui.window),
                                FileChooserAction::Open,
                                None,
                                None,
                            )
                        }
                    };

                    chooser.run();

                    let filename = match chooser.get_filename() {
                        None => return,
                        Some(f) => f,
                    };

                    if let Some(up) = target_rc.upgrade() {
                        let gui = up.borrow();
                        choose_target(&gui.window, gui.plug.clone(), filename);
                    }
                });

                new.window.set_title("Healslut");
                new.window.set_position(gtk::WindowPosition::Center);
                new.window.set_default_size(350, 70);

                new.top_box.add(&h_box);
                h_box.pack_start(&new.btn_target, true, true, 0);
                h_box.pack_end(&new.btn_device, true, true, 0);

                new.top_box.pack_end(&new.statusbar, false, true, 0);

                let id = new.statusbar.get_context_id("foo");
                new.statusbar.push(id, "Foo!");

                new.window.add(&new.top_box);

                new.window.show_all();
            }

            *gui = Some(rc);
        });
    }

    pub fn send<F>(func: F)
    where
        F: 'static + Send + FnOnce(&mut Gui),
    {
        let ctx = MainContext::default();
        ctx.invoke(|| {
            GUI.with(|outer| {
                let mut outer_ref = outer.borrow_mut();
                let inner = outer_ref.as_mut().unwrap();
                let mut inner_ref = inner.borrow_mut();
                func(&mut inner_ref);
            });
        });
    }

    fn event(&mut self, event: Event) {
        eprintln!("Plug -> UI : {:?}", event);

        match event {
            Event::StartScan => {
                self.scanning = true;
                self.statusbar.remove_all(self.status_scan);
                self.statusbar.push(self.status_scan, "Scanning...");
            }

            Event::StopScan => {
                self.scanning = false;
                self.statusbar.remove_all(self.status_scan);
            }

            Event::Connect(name) => {
                self.connected = true;

                self.statusbar.remove_all(self.status_connect);

                let msg = format!("Connected to {}", name);
                self.statusbar.push(self.status_connect, &msg);
            }

            Event::Disconnect => {
                self.connected = false;
                self.statusbar.remove_all(self.status_connect);
            }
        }

        match (self.scanning, self.connected, event) {
            (true, _, Event::Disconnect) | (_, false, Event::StartScan) => {
                self.btn_device.set_label(Self::STOP);
            }

            (false, _, Event::Disconnect) | (_, false, Event::StopScan) => {
                self.btn_device.set_label(Self::SCAN);
            }

            (_, _, Event::Connect(_)) => {
                self.btn_device.set_label(Self::DISCONNECT);
            }

            (_, _, Event::StartScan) => (),
            (_, true, Event::StopScan) => (),
        }
    }

    pub fn connected(&mut self, name: &str) {
        self.event(Event::Connect(name));
    }

    pub fn disconnected(&mut self) {
        self.event(Event::Disconnect);
    }

    pub fn start_scan(&mut self) {
        self.event(Event::StartScan);
    }

    pub fn stop_scan(&mut self) {
        self.event(Event::StopScan);
    }
}

fn choose_target(parent: &ApplicationWindow, plug: Plug, image: PathBuf) {
    // TODO: There are so many problems with this code...

    let window = WindowBuilder::new()
        .transient_for(parent)
        .title("Healslut Target")
        .type_(WindowType::Toplevel)
        .build();

    let evt_box = EventBox::new();
    window.add(&evt_box);

    let img = Image::new();

    let pixbuf = Pixbuf::from_file(&image).unwrap();

    let click_pixbuf = pixbuf.clone();
    let click_img = img.clone();
    let click_window = window.clone();
    evt_box.connect_button_release_event(move |_, event| {
        let alloc = click_img.get_allocation();
        let w = alloc.width as f64;
        let h = alloc.height as f64;

        let (x, y) = event.get_position();

        let o_w = click_pixbuf.get_width() as f64;
        let o_h = click_pixbuf.get_height() as f64;

        let s_x = ((x / w) * o_w).round() as i32;
        let s_y = ((y / h) * o_h).round() as i32;

        choose_target_details(
            plug.clone(),
            click_window.clone(),
            click_pixbuf.clone(),
            s_x,
            s_y,
        );

        Inhibit(true)
    });

    img.connect_size_allocate(move |img, alloc| {
        // TODO: This causes the window to grow slowly over time...
        let scaled = pixbuf
            .scale_simple(alloc.width, alloc.height, InterpType::Hyper)
            .unwrap();
        img.set_from_pixbuf(Some(&scaled));
    });

    evt_box.add(&img);

    window.show_all();
    window.fullscreen();
}

#[derive(Debug, Clone)]
struct Details {
    src: Rc<Capture>,
    x: usize,
    y: usize,

    size: gtk::SpinButton,
    channels: gtk::ComboBoxText,
    threshold: gtk::SpinButton,
    count: gtk::SpinButton,
    image: gtk::Image,

    plug: Plug,
}

impl Details {
    fn target(&self) -> Option<TargetSpec> {
        let channel =
            Channel::from_str(&self.channels.get_active_text()?).unwrap();
        let size = self.size.get_value() as usize;
        let threshold = self.threshold.get_value() as u8;
        let count = self.count.get_value() as usize;

        Some(TargetSpec {
            x: self.x,
            y: self.y,
            channel,
            count,
            size,
            threshold,
        })
    }

    fn save(&self) -> bool {
        if let Some(spec) = self.target() {
            let mask = self.src.extract(&spec);
            self.plug.set_target(Target { spec, mask });
            true
        } else {
            false
        }
    }

    fn preview(&self) {
        match self.try_preview() {
            Some(pb) => {
                self.image.set_from_pixbuf(Some(&pb));
            }
            None => {
                self.image.set_from_pixbuf(None);
            }
        }
    }

    fn try_preview(&self) -> Option<Pixbuf> {
        let target = self.target()?;

        let dims = self.src.dims(&target);
        let w = (dims.x1 - dims.x0) as i32;
        let h = (dims.y1 - dims.y0) as i32;

        let bits = self.src.extract(&target);
        let pixbuf = Pixbuf::new(Colorspace::Rgb, true, 8, w, h)?;

        let mut count = 0;
        for y in 0..h {
            for x in 0..w {
                const M: u8 = u8::max_value();
                let bit = (y * w + x) as usize;

                if bits[bit] {
                    if count <= target.count {
                        pixbuf.put_pixel(x as u32, y as u32, M, M, M, M);
                    } else {
                        pixbuf.put_pixel(
                            x as u32,
                            y as u32,
                            M / 2,
                            M / 2,
                            M / 2,
                            M,
                        );
                    }
                    count += 1;
                } else {
                    pixbuf.put_pixel(x as u32, y as u32, 0, 0, 0, M);
                }
            }
        }

        Some(pixbuf)
    }
}

fn choose_target_details(
    plug: Plug,
    window: Window,
    src: Pixbuf,
    x: i32,
    y: i32,
) {
    // TODO: Figure out why gtk::Popover doesn't work here.

    let deets = DialogBuilder::new()
        .modal(true)
        .transient_for(&window)
        .title("Healslut Target Specification")
        .build();

    deets.add_button("Cancel", gtk::ResponseType::Cancel);
    deets.add_button("Save", gtk::ResponseType::Accept);

    let content = deets.get_content_area();

    let grid = gtk::Grid::new();
    content.add(&grid);
    grid.set_column_spacing(10);
    grid.set_row_spacing(10);

    let image = gtk::Image::new();
    grid.attach(&image, 0, 0, 2, 2);

    let size_lbl = gtk::Label::new(Some("Side Length"));
    grid.attach(&size_lbl, 0, 2, 1, 1);

    let size = gtk::SpinButton::with_range(1., 100., 1.);
    grid.attach(&size, 1, 2, 1, 1);
    size.set_hexpand(true);
    size.set_value(50.);
    size.set_tooltip_text(Some("Size of the icon"));

    let channels_lbl = gtk::Label::new(Some("Channel"));
    grid.attach(&channels_lbl, 0, 3, 1, 1);

    let channels = gtk::ComboBoxText::new();
    grid.attach(&channels, 1, 3, 1, 1);
    channels.set_hexpand(true);
    channels.append_text("Red");
    channels.append_text("Green");
    channels.append_text("Blue");
    channels.append_text("Cyan");
    channels.append_text("Magenta");
    channels.append_text("Yellow");
    channels.append_text("Black");

    let threshold_lbl = gtk::Label::new(Some("Threshold"));
    grid.attach(&threshold_lbl, 0, 4, 1, 1);

    let threshold = gtk::SpinButton::with_range(0., 255., 1.);
    grid.attach(&threshold, 1, 4, 1, 1);
    threshold.set_hexpand(true);
    threshold.set_value(128.);
    threshold.set_tooltip_text(Some("Find silhouette on black background"));

    let count_lbl = gtk::Label::new(Some("Tolerance"));
    grid.attach(&count_lbl, 0, 5, 1, 1);

    let count = gtk::SpinButton::with_range(1., 2500., 1.);
    grid.attach(&count, 1, 5, 1, 1);
    count.set_hexpand(true);
    count.set_value(50.);
    count.set_tooltip_text(Some("Smaller values are more strict"));

    let count_clone = count.clone();
    size.connect_value_changed(move |sender| {
        let value = sender.get_value();
        let max = value * value;
        count_clone.get_adjustment().set_upper(max);

        if count_clone.get_value() > max {
            count_clone.set_value(max);
        }
    });

    let details = Details {
        x: x as usize,
        y: y as usize,
        src: Rc::new(Capture::try_from(src).unwrap()),
        channels,
        count,
        size,
        threshold,
        image,
        plug,
    };

    details.preview();

    let evt_details = details.clone();
    details
        .channels
        .connect_changed(move |_| evt_details.preview());

    let evt_details = details.clone();
    details
        .count
        .connect_changed(move |_| evt_details.preview());

    let evt_details = details.clone();
    details.size.connect_changed(move |_| evt_details.preview());

    let evt_details = details.clone();
    details
        .threshold
        .connect_changed(move |_| evt_details.preview());

    deets.connect_close(move |_| {
        window.close();
    });

    let resp_deets = deets.clone();
    deets.connect_response(move |_, resp| {
        if resp == gtk::ResponseType::Accept && !details.save() {
            return;
        }

        resp_deets.emit_close();
    });

    deets.show_all();
}
