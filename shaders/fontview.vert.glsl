#version 420

in vec2 vp;
in vec2 vt;
out vec2 st;


void main () {
    st = vt;
    gl_Position = vec4 (vp, 0.0, 1.0);
}
