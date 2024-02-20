struct Uniforms {
	uint seed; 
	uint samples;
	uint bounces;
	uint dummy;

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

struct Material {
	vec4 baseColor;
	vec4 emissive;
	float roughness;
	float metallic;
};

struct HitInfo {
	vec3 pos;
	vec3 normal;
	Material material;
};
