use iceoryx2::prelude::ZeroCopySend;

pub const SERVICE_NAME_HEADING_ERROR: &str = "nsm/heading_error";
pub const SERVICE_NAME_ABS_LINE_GRADIENT: &str = "nsm/abs_line_gradient";
pub const SERVICE_NAME_CORNER_DETECTED: &str = "nsm/corner_detected";
pub const SERVICE_NAME_CORNER_DIRECTION: &str = "nsm/corner_direction";
pub const SERVICE_NAME_CORNER_POINT: &str = "nsm/corner_point";

// IPC types
#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
pub struct HeadingErrorMsg {
    pub valid: u8,
    pub value: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
pub struct AbsLineGradientMsg {
    pub valid: u8,
    pub value: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
pub struct CornerDetectedMsg {
    pub detected: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
pub struct CornerDirectionMsg {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, ZeroCopySend)]
pub struct CornerPointMsg {
    pub x: f32,
    pub y: f32,
}
