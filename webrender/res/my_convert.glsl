// version 300 es

#extension GL_ARB_explicit_attrib_location : require
#extension GL_ARB_separate_shader_objects : require
#define WR_FEATURE_TEXTURE_2D

#include shared,shared_other

#ifdef WR_VERTEX_SHADER

precision mediump float;
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texture;
out vec2 v_texture;

void main() {
   gl_Position = vec4(a_position, 0.0, 1.0);
   v_texture = a_texture;
}

#endif


#ifdef WR_FRAGMENT_SHADER

precision mediump float;
in vec2 v_texture;
layout(location = 0) out vec4 outColor0;
layout(location = 1) out vec4 outColor1;
layout(location = 2) out vec4 outColor2;
layout(location = 3) out vec4 outColor3;

uniform int yuv_id;
uniform sampler2D tex_argb;

float Y(vec3 c)  {
  float result = (0.257 * c.r) + (0.504 * c.g) + (0.098 * c.b) + 0.0625;
  return result;
}

float U(vec3 c) {
    float result = -(0.148 * c.r) - (0.291 * c.g) + (0.439 * c.b) + 0.5;
    return result;
}

float V(vec3 c) {
  float result = (0.439 * c.r) - (0.368 * c.g) - (0.071 * c.b) + 0.5; 
  return result;
}

void main() {
  vec3 col = texture(tex_argb, v_texture).rgb;
  if(yuv_id == 0) {
    outColor1 = vec4(Y(col), 1.0, 1.0, 1.0);
  } else if (yuv_id == 1) {
    outColor2 = vec4(U(col), 1.0, 1.0, 1.0);
  } else {
    outColor3 = vec4(V(col), 1.0, 1.0, 1.0);
  }
}

#endif