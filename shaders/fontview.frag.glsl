#version 420

in vec2 st;
uniform sampler2D tex;
uniform vec4 text_colour;
out vec4 frag_colour;


void main () {
    frag_colour = texture (tex, st) * text_colour;
}
