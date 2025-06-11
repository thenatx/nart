use bytemuck::{Pod, Zeroable};
use log::debug;
use std::marker::PhantomData;
use wgpu::{util::DeviceExt, Buffer, BufferDescriptor, BufferUsages, Device, Queue};

pub struct GpuBuffer<T: Pod + Zeroable> {
    buffer: Buffer,
    capacity: usize,
    usage: BufferUsages,
    label: String,
    _phantom: PhantomData<T>,
}

impl<T: Pod + Zeroable> GpuBuffer<T> {
    pub fn new(
        device: &Device,
        usage: BufferUsages,
        label: &str,
        initial_data: Option<&[T]>,
    ) -> Self {
        let capacity = initial_data.map(|d| d.len()).unwrap_or(1);
        let size = (capacity * std::mem::size_of::<T>()) as wgpu::BufferAddress;

        let buffer = if let Some(data) = initial_data {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage,
            })
        } else {
            device.create_buffer(&BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: initial_data.is_some(),
            })
        };

        Self {
            buffer,
            capacity,
            usage,
            label: label.to_string(),
            _phantom: PhantomData,
        }
    }

    pub fn write(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        let required_capacity = data.len();

        if required_capacity > self.capacity {
            self.resize(device, self.capacity + required_capacity * 3 / 2);
        }

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    pub fn resize(&mut self, device: &Device, new_capacity: usize) {
        if new_capacity <= self.capacity {
            return;
        }

        let new_size = (new_capacity * std::mem::size_of::<T>()) as wgpu::BufferAddress;
        let new_buffer = device.create_buffer(&BufferDescriptor {
            label: Some(&self.label),
            size: new_size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        self.buffer = new_buffer;
        self.capacity = new_capacity;
        debug!(
            "Buffer with label '{}' resized to {} bytes",
            self.label, new_size
        );
    }

    pub fn inner(&self) -> &Buffer {
        &self.buffer
    }
}

pub struct VertexBuffer<T: Pod + Zeroable>(GpuBuffer<T>);

impl<T: Pod + Zeroable> VertexBuffer<T> {
    pub fn new(device: &Device, label: &str, vertices: Option<&[T]>) -> Self {
        Self(GpuBuffer::new(
            device,
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            label,
            vertices,
        ))
    }

    pub fn write(&mut self, device: &Device, queue: &Queue, data: &[T]) {
        self.0.write(device, queue, data);
    }

    pub fn raw_buffer(&self) -> &Buffer {
        self.0.inner()
    }
}
