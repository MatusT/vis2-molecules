use crate::application::*;

use iced_wgpu::Renderer;
use iced_winit::{slider, Column, Container, Element, Length, Slider, Space, Text};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    SolventRadiusChanged(f32),
    MaxNeighboursChanged(f32),
}

pub struct UserInterface {
    solvent_radius_slider: slider::State,
    max_neighbours_slider: slider::State,
}

impl UserInterface {
    pub fn new() -> Self {
        Self {
            solvent_radius_slider: iced_wgpu::slider::State::new(),
            max_neighbours_slider: iced_wgpu::slider::State::new(),
        }
    }

    pub fn update(&self, message: Message, application: &mut Application) {
        match message {
            Message::SolventRadiusChanged(solvent_radius) => {
                application.set_solvent_radius(solvent_radius);
            }
            Message::MaxNeighboursChanged(max_neighbours) => {
                application.set_max_neighbours(max_neighbours.round() as i32);
            }
        };
    }

    pub fn view<'a>(&'a mut self, application: &Application) -> Element<'a, Message, Renderer> {
        Container::new(
            Column::new()
                .push(Text::new("Options").size(24))
                .push(Space::new(Length::Fill, Length::Units(12)))
                .push(Text::new("Solvent radius").size(18))
                .push(Slider::new(
                    &mut self.solvent_radius_slider,
                    0.0..=2.0,
                    application.solvent_radius(),
                    move |n| Message::SolventRadiusChanged(n),
                ))
                .push(Text::new("Max neighbours").size(18))
                .push(Slider::new(
                    &mut self.max_neighbours_slider,
                    1.0..=45.0,
                    application.max_neighbours() as f32,
                    move |n| Message::MaxNeighboursChanged(n),
                ))
                .padding(12),
        )
        .width(Length::Units(200))
        .into()
    }
}
