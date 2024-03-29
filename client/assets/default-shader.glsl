#[description]
Default shader used when no postprocessing shader is set up

#[vertex]
#version 450 core

layout (location = 0) in vec2 a_Position;
layout (location = 1) in vec2 a_FrameCoord;

out vec2 v_FrameCoord;

void main() {
    v_FrameCoord = a_FrameCoord;
    gl_Position = vec4(a_Position, 0.0, 1.0);
}

#[fragment]
#version 450 core

in vec2 v_FrameCoord;

uniform sampler2D u_Frame;

layout (location = 0) out vec4 r_Color;

void main() {
    r_Color = texture(u_Frame, v_FrameCoord);
}
