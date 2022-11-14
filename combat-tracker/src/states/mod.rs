use anyhow::Result;
use crossterm::event::Event;

use crate::Frame;

pub trait Boxable {
    fn boxed(self) -> StateBox;
    fn box_clone(&self) -> StateBox;
}

impl<T> Boxable for T
where
    T: 'static + State + Clone,
{
    fn boxed(self) -> StateBox {
        Box::new(self)
    }

    fn box_clone(&self) -> StateBox {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn State> {
    fn clone(&self) -> Box<dyn State> {
        self.box_clone()
    }
}

pub trait State: Boxable {
    fn process(self: Box<Self>, ev: Event) -> Result<StateBox>;
    fn render(&mut self, f: &mut Frame);
}

pub type StateBox = Box<dyn State>;

pub mod insert;
pub use insert::Insert;

pub mod normal;
pub use normal::Normal;

pub mod msg;
pub use msg::Msg;

pub mod fighting;
pub use fighting::Fighting;
