#[description]
default shader for the FlatTexture material

#[vertex]
#version 450 core

layout (location = 0) in vec2 a_Position;
layout (location = 1) in vec2 a_TexCoord;
layout (location = 2) in int a_TexIdx;

uniform mat4 u_ViewProjection;

out vec2 v_TexCoord;
flat out int v_TexIdx;

void main() {
    v_TexCoord = a_TexCoord;
    v_TexIdx = a_TexIdx;
    gl_Position = u_ViewProjection * vec4(a_Position, 0.0, 1.0);
}

#[fragment]
#version 450 core

in vec2 v_TexCoord;
flat in int v_TexIdx;

uniform sampler2D u_Textures[16];

layout (location = 0) out vec4 r_Color;

void main() {
    r_Color = texture(u_Textures[v_TexIdx], v_TexCoord);
}
