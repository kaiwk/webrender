// version 300 es

#extension GL_ARB_explicit_attrib_location : require
#include shared,shared_other

#ifdef WR_VERTEX_SHADER

layout(location = 0) in vec2 a_position;

uniform vec2 u_canvas_info;
uniform float u_scale;
uniform vec2 u_translate;
uniform vec2 u_rotation_center;
uniform vec2 u_rotation_sin_cos;

// rotation -> scale -> translate -> covert space
void main() {
    // mat2 rotation_matrix = mat2(
    //     u_rotation_sin_cos.y, -1.0 * u_rotation_sin_cos.x,
    //     u_rotation_sin_cos.x, u_rotation_sin_cos.y
    // );

    mat2 rotation_matrix = mat2(
        vec2(u_rotation_sin_cos.y, u_rotation_sin_cos.x),
        vec2(-1.0 * u_rotation_sin_cos.x, u_rotation_sin_cos.y)
    );

    // before rotation move shape to the origin position of coordinates
    vec2 beforeRotate = a_position - u_rotation_center;

    // rotate
    vec2 whenRotate = rotation_matrix * beforeRotate;

    // after rotation move shape back
    vec2 afterRotate = whenRotate + u_rotation_center;

    // scale
    vec2 afterScale = afterRotate * u_scale;

    // translate
    vec2 afterTranslate = afterScale + u_translate;

    // convert the rectangle from pixel to 0.0 to 1.0
    vec2 zeroToOne = afterTranslate / u_canvas_info;

    // convert from 0->1 to 0 ->2
    vec2 zeroToTwo = zeroToOne * 2.0;

    // convert from 0->2 to -1->+1(clipspace)
    vec2 clipSpace = zeroToTwo - 1.0;

    // upside down clipSpace = clipSpace * vec2(1, -1) * vec2(1, -1)
    gl_Position = vec4(clipSpace, 0.0, 1.0);
}

#endif

#ifdef WR_FRAGMENT_SHADER

#include shared,shared_other

precision mediump float;

void main() {
    write_output(vec4(1.0, 1.0, 1.0, 1.0));
}

#endif