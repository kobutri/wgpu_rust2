#version 450

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outColorDebug;

layout(std140, set = 0, binding = 0) uniform Uniforms {
    vec3 view_pos;
    vec3 view_dir;
    vec3 view_up;
    vec3 view_right;
    float fov;
    int width;
    int height;
    float octree_size;
    int octree_depth;
} uniforms;

struct Node {
    int material_id;
    int sub_voxels[8];
	int ropes[6];
};

layout(std430, set = 1, binding = 0) buffer octree {
    Node data[];
};

#define MAX_DEPTH 12
#define EPS 1e-5
#define big 10e10
const float infinity = 1. / 0.;


const int SOLID = 1;
const int EMPTY = 0;

struct Ray {
    vec3 origin;
    vec3 dir;
    vec3 invDir;
};

struct StackNode {
	vec3 origin;
	int index;
	int hit;
	int subvoxel_index;
};

// global state
float size = uniforms.octree_size;
StackNode stack[12];
int currentStackIndex = 0;
StackNode currentStack = StackNode(vec3(0), 0, 0, 0);

Ray generate_ray()  {
    float x_ratio = float(gl_FragCoord.x) / float(uniforms.width);
    float y_ratio = float(gl_FragCoord.y) / float(uniforms.height);
    float aspect = float(uniforms.width) / float(uniforms.height);
    float a = tan(uniforms.fov/2.0);
    float a2 = a/aspect;
    vec3 view_center = uniforms.view_pos + uniforms.view_dir;

    vec3 target = view_center + uniforms.view_right * a * (2 * x_ratio - 1) + uniforms.view_up * a2 * (2 * y_ratio - 1);
    vec3 dir = normalize(target-uniforms.view_pos);
    return Ray(uniforms.view_pos, dir, vec3(1.0/dir.x, 1.0/dir.y, 1.0/dir.z));
}

float max_component(vec3 v) {
	return max(v.x, max(v.y, v.z));
}

float min_component(vec3 v) {
	return min(v.x, min(v.y, v.z));
}


vec3 sortVec3( vec3 v )
{
   	vec2 t = vec2(min(v.x, v.y), max(v.x, v.y));
	return vec3(min(t.x, v.z), min(max(v.z, t.x), t.y), max(t.y, v.z)); 
}

ivec3 getFaces(ivec3 n, vec3 old, vec3 new) {
	return ivec3(
		int((new.x == old.x))*(n.x) + int(new.x == old.y)*(n.y) + int(new.x == old.z)*(n.z),
		int((new.y == old.x))*(n.x) + int(new.y == old.y)*(n.y) + int(new.y == old.z)*(n.z),
		int((new.z == old.x))*(n.x) + int(new.z == old.y)*(n.y) + int(new.z == old.z)*(n.z)
	);
}

int getFace(bvec3 n, vec3 old, float t) {
	return int((t == old.x))*int(n.x) + 2*int(t == old.y)*int(n.y) + 4*int(t == old.z)*int(n.z);
}

int getSubvoxel(float tsMin, vec3 tZero, vec3 dir) {
	bvec3 positive = lessThanEqual(tZero, vec3(tsMin));
	ivec3 signs = 1-ivec3((sign(dir) + 1)/ 2);
	return 1*(int(positive.x)^signs.x) + 2*(int(positive.y)^signs.y) + 4*(int(positive.z)^signs.z);
}

int intersect(Ray r) {
	vec3 point = r.origin - currentStack.origin;
	vec3 tMinus = (-size-point)*r.invDir;
	vec3 tPlus = (size-point)*r.invDir;
	
	// if hit
	vec3 tMin = min(tMinus, tPlus);
	bvec3 tMinMask = equal(tMin, tPlus);
	vec3 tMax = max(tMinus, tPlus);
	bvec3 tMaxMask = equal(tMin, tPlus);
	
	float tsMin = max_component(tMin);
	
	float tsMax = min_component(tMax);
	
	if (tsMin <= tsMax) {
		vec3 tZero = (-point)*r.invDir;
		int subvoxel = getSubvoxel(tsMin, tZero, r.dir);
		
		vec3 tZeroSorted = sortVec3(tZero);
		ivec3 tZeroFacesSorted = getFaces(ivec3(1,2,4), tZero, tZeroSorted);
		int result = 1 | 2 | (subvoxel << 4);
		bvec3 subvoxelsValid = equal(lessThanEqual(tZeroSorted, vec3(tsMax)), greaterThanEqual(tZeroSorted, vec3(tsMin)));
		int j = 0;
		if(subvoxelsValid.x) {
			subvoxel = subvoxel ^ tZeroFacesSorted.x;
			result |= subvoxel << (8+j*4);
			j++;
		}
		if(subvoxelsValid.y) {
			subvoxel = subvoxel ^ tZeroFacesSorted.y;
			result |= subvoxel << (8+j*4);
			j++;
		}
		if(subvoxelsValid.z) {
			subvoxel = subvoxel ^ tZeroFacesSorted.z;
			result |= subvoxel << (8+j*4);
			j++;
		}
		result |= 8 << (8 + j*4);
		return result;
		
	} else {
		return 1;
	}
	
}

int getNthSubvoxel(int hit, int index) {
	int subvoxel = ((hit >> 4) >> (4*index));
	if ((subvoxel & 8) != 0) {
		return -1;
	} else {
		return subvoxel & 7;
	}
}

#define 

struct HitResult {
	int hit;
	int data; // i
};

HitResult intersect(Ray r) {

}



void main()
{	
	Ray ray = generate_ray();
	bool hit = false;
	int i = 0;
	for(; i < 100000; i++) {
	
	}
    outColor = vec4(vec3(i >= 100000, hit, 0), 1.0);
}

