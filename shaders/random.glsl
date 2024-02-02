// 3 Dimensional PRNG based on http://www.jcgt.org/published/0009/03/02/
uvec3 pcg3d(inout uvec3 v) {
    v = v * 1664525u + 1013904223u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v ^= v >> 16u;
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    return v;
}

// Generates a random directional vector in the area around a unit hemisphere
// r is 2 uniformly random floats in the range (0, 1)
// Mathematical derivation found in Design documentation and
// Chapter 13.6.1 of the PBR book
// https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations
// The PDF of this function is constant : 1 / 2PI
vec3 uniformSampleHemisphere(vec2 r) {
    float s = sqrt(1 - r.x * r.x);
    float phi = 2 * PI * r.y;
    return vec3(s * cos(phi), r.x, s * sin(phi));
}