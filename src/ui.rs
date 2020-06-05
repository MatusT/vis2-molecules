//!
//! Description of the UI.
//!


use crate::application::*;

use iced_wgpu::Renderer;
use iced_winit::{slider, Column, Container, Element, Length, Slider, Space, Text};

/// Events that can be fired by the UI
#[derive(Debug, Clone, Copy)]
pub enum Message {
    /// Called when solvent radius is adjusted
    SolventRadiusChanged(f32),
    /// Called when number of maximum neighbours is adjusted
    MaxNeighboursChanged(f32),
    /// Called when maximum number of steps per frame is adjusted
    MaxStepsChanged(f32),
}
/// State of the user interface 
pub struct UserInterface {
    solvent_radius_slider: slider::State,
    max_neighbours_slider: slider::State,
    max_steps_slider: slider::State,
}

impl UserInterface {
    /// Initializes default state of the UI
    pub fn new() -> Self {
        Self {
            solvent_radius_slider: iced_wgpu::slider::State::new(),
            max_neighbours_slider: iced_wgpu::slider::State::new(),
            max_steps_slider: iced_wgpu::slider::State::new(),
        }
    }

    /// Processes a fired event
    pub fn update(&self, message: Message, application: &mut Application) {
        match message {
            Message::SolventRadiusChanged(solvent_radius) => {
                application.set_solvent_radius(solvent_radius);
            }
            Message::MaxNeighboursChanged(max_neighbours) => {
                application.set_max_neighbours(max_neighbours.round() as i32);
            }
            Message::MaxStepsChanged(max_steps) => {
                application.set_max_steps(max_steps.round() as i32);
            }
        };
    }

    /// Returns the UI based on a state
    pub fn view<'a>(&'a mut self, application: &Application) -> Element<'a, Message, Renderer> {
        Container::new(
            Column::new()
                .push(Text::new("Options").size(24))
                .push(Space::new(Length::Fill, Length::Units(12)))
                .push(Text::new(format!("Solvent radius: {:.2}", application.solvent_radius())).size(18))
                .push(Slider::new(
                    &mut self.solvent_radius_slider,
                    0.0..=2.0,
                    application.solvent_radius(),
                    move |n| Message::SolventRadiusChanged(n),
                ))
                .push(Text::new("Max neighbours: ".to_string() + &application.max_neighbours().to_string()).size(18))
                .push(Slider::new(
                    &mut self.max_neighbours_slider,
                    1.0..=45.0,
                    application.max_neighbours() as f32,
                    move |n| Message::MaxNeighboursChanged(n),
                ))
                .push(Text::new(format!("Max steps per frame: {}", application.max_steps() as u32)).size(18))
                .push(Slider::new(
                    &mut self.max_steps_slider,
                    1.0..=64.0,
                    application.max_steps() as f32,
                    move |n| Message::MaxStepsChanged(n),
                ))
                .padding(12),
        )
        .width(Length::Units(200))
        .into()
    }
}
