use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;

// We will embed this inside runtime, but for now let's just make it compilable.
