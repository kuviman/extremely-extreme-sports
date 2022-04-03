varying vec2 v_vt;
varying float v_opacity;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
attribute vec2 i_pos;
attribute vec2 i_vel;
attribute float i_time;
attribute float i_size;
attribute float i_opacity;
uniform float u_time;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
void main() {
    v_opacity = (1.0 - (u_time - i_time)) * i_opacity;
    v_vt = (a_pos + 1.0) / 2.0;
    vec3 pos = u_projection_matrix * u_view_matrix * vec3(a_pos * i_size + i_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform vec4 u_color;
void main() {
    gl_FragColor = vec4(0.8, 0.8, 0.85, 0.7 * texture2D(u_texture, v_vt).w);
    gl_FragColor.w *= v_opacity;
}
#endif