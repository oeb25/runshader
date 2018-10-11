#version 330 core

out vec4 FragColor;

in vec2 TexPos;

uniform float time;

void main() {
	vec3 color = vec3(TexPos, abs(sin(time)));
	FragColor = vec4(color, 1.0);
}