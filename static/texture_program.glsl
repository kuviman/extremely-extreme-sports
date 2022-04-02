varying vec2 v_vt;

#ifdef VERTEX_SHADER
attribute vec2 a_pos;
uniform mat3 u_projection_matrix;
uniform mat3 u_view_matrix;
uniform mat3 u_model_matrix;
void main() {
    v_vt = (a_pos + 1.0) / 2.0;
    vec3 pos = u_projection_matrix * u_view_matrix * u_model_matrix * vec3(a_pos, 1.0);
    gl_Position = vec4(pos.xy, 0.0, pos.z);
}
#endif

#ifdef FRAGMENT_SHADER
uniform sampler2D u_texture;
uniform vec4 u_color;
void main() {
    gl_FragColor = texture2D(u_texture, v_vt) * u_color;
    if (gl_FragColor.w < 0.5) {
        discard;
    }
}
#endif