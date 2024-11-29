use std::future::Future;

// The extension traits that make use of the Project trait are prone to raising
// issues related to issue #100013. In this case, this is due to the compiler
// being unable to confirm an inner Future is also Send. This trait works around
// the issue, allowing the compiler to confirm the Future is actually Send.
pub trait IntoSendFuture: Future + Send {
	fn into_send_future(self) -> impl Future<Output = Self::Output> + Send;
}

impl<T> IntoSendFuture for T
where
	T: Future + Send,
{
	fn into_send_future(self) -> impl Future<Output = Self::Output> + Send {
		self
	}
}
