// version 300 es

#extension GL_ARB_separate_shader_objects : require
#extension GL_ARB_explicit_attrib_location : require
#define WR_FEATURE_TEXTURE_2D

#include shared,shared_other

#ifdef WR_VERTEX_SHADER

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texture;

uniform vec2 u_canvas_info;
uniform vec2 u_translate;

out vec2 v_position;
out vec2 v_texture;

// translate -> covert space
void main() {
  v_texture = a_texture;

  // translate
  vec2 afterTranslate = a_position + u_translate;

  // convert the rectangle from pixel to 0.0 to 1.0
  vec2 zeroToOne = afterTranslate / u_canvas_info;

  // convert from 0->1 to 0 ->2
  vec2 zeroToTwo = zeroToOne * 2.0;

  // convert from 0->2 to -1->+1(clipspace)
  vec2 clipSpace = zeroToTwo - 1.0;

  // output
  v_position = clipSpace * vec2(1, -1);

  // upside down clipSpace = clipSpace * vec2(1, -1) * vec2(1, -1)
  gl_Position = vec4(clipSpace, 0.0, 1.0);
}

#endif


#ifdef WR_FRAGMENT_SHADER

precision mediump float;

// out vec4 FragColor;

in vec2 v_texture;
in vec2 v_position;

// texture
uniform sampler2D tex_rgba;
uniform sampler2D tex_y;
uniform sampler2D tex_u;
uniform sampler2D tex_v;
uniform sampler2D tex_a;

// base info
uniform vec2 u_canvas_info;// (width, height)
uniform vec4 u_frame_info;// (left, top, width, height)
uniform bool is_argb;
uniform bool is_rgba;
uniform bool yuv_has_alpha;

// effect info
uniform bool is_mirror;
uniform bool is_round;
uniform bool is_circle;
uniform float round_radius;
uniform float circle_radius;

void main(void)
{
    vec2 texCoord = v_texture;
    bool trans = false;
    if (is_round || is_circle) {
        float half_cw = u_canvas_info.x / 2.0;
        float half_ch = u_canvas_info.y / 2.0;
        float frame_center_x = u_frame_info.x + u_frame_info.z / 2.0;
        float frame_center_y = u_frame_info.y - u_frame_info.w / 2.0;
        float half_fw = u_frame_info.z / 2.0;
        float half_fh = u_frame_info.w / 2.0;

        vec2 xy = vec2(v_position.x * half_cw - frame_center_x, v_position.y * half_ch - frame_center_y);

        // 圆角
        if (is_round) {
            vec2 xy1 = vec2(round_radius - half_fw, half_fh - round_radius);
            vec2 xy2 = vec2(half_fw - round_radius, half_fh - round_radius);
            vec2 xy3 = vec2(half_fw - round_radius, round_radius - half_fh);
            vec2 xy4 = vec2(round_radius - half_fw, round_radius - half_fh);
            if (xy.x < xy1.x && xy.y > xy1.y && length(vec2(xy.x - xy1.x, xy.y - xy1.y)) > round_radius) {
                trans = true;
            }
            if (xy.x > xy2.x && xy.y > xy2.y && length(vec2(xy.x - xy2.x, xy.y - xy2.y)) > round_radius) {
                trans = true;
            }
            if (xy.x > xy3.x && xy.y < xy3.y && length(vec2(xy.x - xy3.x, xy.y - xy3.y)) > round_radius) {
                trans = true;
            }
            if (xy.x < xy4.x && xy.y < xy4.y && length(vec2(xy.x - xy4.x, xy.y - xy4.y)) > round_radius) {
                trans = true;
            }
        }

        // 圆
        if (is_circle) {
            if (length(xy) >= circle_radius) {
                trans = true;
            }
        }
    }

    if (trans) {
        discard;
    } else {
        if (is_mirror) {
            texCoord.x = (1.0 - v_texture.x);
        }

        vec3 rgb = vec3(0, 0, 0);
        float a = 1.0;
        if (is_argb) {
            rgb = texture(tex_rgba, texCoord).bgr;
            a = texture(tex_rgba, texCoord).a;
        } else if (is_rgba) {
            rgb = texture(tex_rgba, texCoord).rgb;
            a = texture(tex_rgba, texCoord).a;
        } else {
            vec3 yuv;
            yuv.x = texture(tex_y, texCoord).r;
            yuv.y = texture(tex_u, texCoord).r - 0.5;
            yuv.z = texture(tex_v, texCoord).r - 0.5;
            rgb = mat3(1, 1, 1,
            0, -0.39465, 2.03211,
            1.13983, -0.58060, 0) * yuv;
            if (yuv_has_alpha) {
                a = texture(tex_a, texCoord).r;
            }
        }

        write_output(vec4(rgb, a));
    }
}

#endif