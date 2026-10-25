//! Typed payload queues. Native transport owns channel registration and serialization.
use bevy::prelude::Component;
use std::any::TypeId;

pub struct QueuedMessage<M> {
    pub message: M,
    pub channel: TypeId,
    pub priority: f32,
}

#[derive(Component)]
pub struct MessageSender<M: Send + Sync + 'static> {
    messages: Vec<QueuedMessage<M>>,
}

impl<M: Send + Sync + 'static> Default for MessageSender<M> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl<M: Send + Sync + 'static> MessageSender<M> {
    pub fn send<C: Send + Sync + 'static>(&mut self, message: M) {
        self.send_with_priority::<C>(message, 1.0);
    }

    pub fn send_with_priority<C: Send + Sync + 'static>(&mut self, message: M, priority: f32) {
        self.messages.push(QueuedMessage {
            message,
            channel: TypeId::of::<C>(),
            priority,
        });
    }

    pub fn drain(&mut self) -> impl Iterator<Item = QueuedMessage<M>> {
        self.messages.drain(..)
    }
}

#[derive(Component)]
pub struct MessageReceiver<M: Send + Sync + 'static> {
    messages: Vec<M>,
}

impl<M: Send + Sync + 'static> Default for MessageReceiver<M> {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl<M: Send + Sync + 'static> MessageReceiver<M> {
    pub fn push(&mut self, message: M) {
        self.messages.push(message);
    }

    pub fn receive(&mut self) -> impl Iterator<Item = M> {
        self.messages.drain(..)
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests;
