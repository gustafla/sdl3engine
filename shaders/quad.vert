out vec2 FragCoord;
void main() {
    vec2 c = vec2(-1, 1);
    vec4 coords[4] = vec4[4](c.xxyy, c.yxyy, c.xyyy, c.yyyy);
    FragCoord = coords[gl_VertexID].xy;
    gl_Position = coords[gl_VertexID];
}
