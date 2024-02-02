struct Uniforms {
	uint seed; 
	uint samples;
	uint bounces;
	uint mode;

	float focal_length;
	float aperture;
	float exposure;
	float time;

	vec4 pos;

	mat4 inverseView;
	mat4 inverseProj;
};

struct Globals {
	uvec3 rngState;
};

struct Ray {
	vec3 origin;
	vec3 dir;
};

struct HitInfo {
	vec3 pos;
	vec3 normal;
};