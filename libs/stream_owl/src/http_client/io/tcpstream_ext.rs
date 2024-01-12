use socket2::SockRef;
use std::io;
use std::ops::Deref;
use std::pin::Pin;
use tokio::net::TcpStream;

pub trait TcpStreamExt {
    fn set_send_buf_size(&self, size: usize) -> Result<(), io::Error>;
    fn send_buf_size(&self) -> Result<usize, io::Error>;
}

impl TcpStreamExt for Pin<&mut TcpStream> {
    fn set_send_buf_size(&self, size: usize) -> Result<(), io::Error> {
        let socket = SockRef::from(self.deref());
        socket.set_recv_buffer_size(size)
    }

    fn send_buf_size(&self) -> Result<usize, io::Error> {
        let socket = SockRef::from(self.deref());
        socket.recv_buffer_size()
    }
}

impl TcpStreamExt for TcpStream {
    fn set_send_buf_size(&self, size: usize) -> Result<(), io::Error> {
        let socket = SockRef::from(self);
        socket.set_recv_buffer_size(size)
    }

    fn send_buf_size(&self) -> Result<usize, io::Error> {
        let socket = SockRef::from(self);
        socket.recv_buffer_size()
    }
}
