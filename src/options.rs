use super::display::{Context, Frame};
use crate::wayland::{
    river_layout_unstable_v1::{
        zriver_layout_manager_v1::ZriverLayoutManagerV1, zriver_layout_v1,
        zriver_layout_v1::ZriverLayoutV1,
    },
    river_options_unstable_v1::{
        zriver_option_handle_v1, zriver_option_handle_v1::ZriverOptionHandleV1,
        zriver_options_manager_v1::ZriverOptionsManagerV1,
    },
};
use wayland_client::EventQueue;
use wayland_client::DispatchData;
// use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::Main;

#[derive(Clone, Debug)]
pub struct Options {
    pub state: bool,
    pub serial: u32,
    pub tagmask: u32,
    pub layout: Option<Main<ZriverLayoutV1>>,
    pub view_amount: u32,
    pub usable_width: u32,
    pub usable_height: u32,
    pub view_padding: u32,
    pub output_padding: u32,
    pub main_index: u32,
    pub main_factor: f64,
    pub main_count: u32,
    pub capacity: u32,
    pub arguments: String,
}

#[derive(Copy, Clone)]
pub union Value {
    double: f64,
    uint: u32,
    int: i32,
}

#[derive(Copy, Clone, Debug)]
pub enum Layout {
    Tab,
    Full,
    Vertical,
    Horizontal,
}

impl Options {
    pub fn new() -> Options {
        return {
            Options {
                state: true,
                serial: 0,
                tagmask: 0,
                layout: None,
                view_amount: 0,
                capacity: 0,
                view_padding: 0,
                output_padding: 0,
                usable_width: 0,
                usable_height: 0,
                main_factor: 0.0,
                main_index: 0,
                main_count: 0,
                arguments: String::from("v|h|"),
            }
        }
    }
    // Listen to the options and layout and returns an Options when the context is updated
    pub fn init(mut self,context: Context)->Options {

        let new_context=context.clone();

        self.layout = Some(new_context
            .layout_manager
            // .unwrap()
            .expect("Compositor doesn't implement ZriverOptionsManagerV1")
            .get_river_layout(&new_context.output.unwrap(), new_context.namespace));
        self.clone().layout.unwrap().quick_assign(move |layout_obj, event, mut option: DispatchData| match event {
            zriver_layout_v1::Event::LayoutDemand {
                view_amount,
                usable_width,
                usable_height,
                serial,
            } => {
                option.get::<Options>().unwrap().serial= serial;
                option.get::<Options>().unwrap().view_amount= view_amount;
                option.get::<Options>().unwrap().usable_height= usable_height;
                option.get::<Options>().unwrap().usable_width= usable_width;
            }
            zriver_layout_v1::Event::AdvertiseView {
                tags,
                app_id,
                serial,
            } => {}
            zriver_layout_v1::Event::NamespaceInUse => {
                println!("Namespace already in use.");
                option.get::<Options>().unwrap().state=false;
            }
            zriver_layout_v1::Event::AdvertiseDone { serial } => {}
        });

        self.get_option("main-factor", &context);
        self.get_option("main-count", &context);
        self.get_option("main-index", &context);
        self.get_option("view-padding", &context);
        self.get_option("capacity", &context);
        self.get_option("layout", &context);

        // event_queue
        //     .dispatch(&mut self, |_, _, _| unreachable!())
        //     .unwrap();

        return self;
    }
    fn get_option(&mut self, name: &'static str, context: &Context) {
        let option = context
            .options_manager
            .clone()
            .expect("Compositor doesn't implement ZriverOptionsManagerV1")
            // .unwrap()
            .get_option_handle(name.to_owned(), Some(&context.output.as_ref().unwrap()));
        option.quick_assign(move |_, event,mut option| {
            let mut option_value: Value = Value { uint: 0 };
            let mut args: String = String::new();
            match event {
                zriver_option_handle_v1::Event::StringValue { value } => {
                    args=value.unwrap();
                    // layout_ref.as_mut().unwrap().push_str(&value.unwrap())
                }
                zriver_option_handle_v1::Event::FixedValue { value } => option_value.double = value,
                zriver_option_handle_v1::Event::UintValue { value } => option_value.uint = value,
                zriver_option_handle_v1::Event::IntValue { value } => {
                    if value < 0 {
                        option_value.int = 0;
                    } else {
                        option_value.int = value;
                    }
                }
                zriver_option_handle_v1::Event::Unset => {}
            }
            unsafe {
                match name {
                    "main-index" => option.get::<Options>().unwrap().main_index = option_value.uint,
                    "main-count" => option.get::<Options>().unwrap().main_count = option_value.uint,
                    "main-factor" => option.get::<Options>().unwrap().main_factor = option_value.double,
                    "view-padding" => option.get::<Options>().unwrap().view_padding = option_value.uint,
                    "output-padding" => option.get::<Options>().unwrap().output_padding = option_value.uint,
                    "capacity" => option.get::<Options>().unwrap().capacity = option_value.uint,
                    "layout"=>option.get::<Options>().unwrap().arguments=args,
                    _ => {}
                }
            }
        });
    }
    pub fn parse(&self) -> Vec<Layout> {
        let mut closure=false;
        let mut start:usize=0;
        let mut i :usize=0;
        let mut layout=Vec::with_capacity(self.view_amount as usize);
        let my_chars: Vec<_>=self.arguments.chars().collect();
        while layout.len() < self.view_amount as usize {
            let c=my_chars[i];
            let mut orientation:Layout=Layout::Full;
            match c {
                'v' => orientation=Layout::Vertical,
                'h' => orientation=Layout::Horizontal,
                't' => orientation=Layout::Tab,
                'f' => orientation=Layout::Full,
                '|' => if closure {
                    closure=false;
                    i=start;
                } else {
                    closure=true;
                    start=i;
                }
                _ => println!("{}: Not a valid character at index {}", c, i),
            }
            layout.push(orientation);
            if i < layout.len() {i+=1}
        }
        return layout;
    }
    pub fn push_dimensions(&self, frame:&Frame) {
        self.layout.clone().unwrap().push_view_dimensions(
            self.serial,
            frame.x as i32,
            frame.y as i32,
            frame.w,
            frame.h,
        )
    }
    pub fn commit(&self) {
        self.layout.clone().unwrap().commit(self.serial);
    }
}

