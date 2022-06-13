#version 330

uniform vec2 uResolution;
uniform float uTime;
uniform vec3 uCameraPosition;
uniform vec3 uCameraDirection;
uniform mat4 matV;



out vec4 outColor;

struct Ray {
	vec3 origin;
	vec3 dir;
	vec3 invDir;
};

#define PI 3.14159265359
#define big 10e10

Ray generate_ray() {

	float fov = 45 * PI/180;
	const vec3 global_up = vec3(0, 1, 0);
	
	vec2 uv = gl_FragCoord.xy/uResolution;
	
	
	vec3 dir = normalize(uCameraDirection);
	vec3 right = normalize(cross(dir, global_up));
	vec3 up = normalize(cross(right, dir));
	
	float aspect = uResolution.x/uResolution.y;
	
	vec3 target = uCameraPosition+dir;
	//vec3(0);
	float a = tan(fov/2);
	target += right*a*(2*uv.x-1);
	target += up*a*(2*uv.y-1)/aspect;
	vec3 ray_dir = normalize(target-uCameraPosition);
	

	
	return Ray(uCameraPosition, ray_dir, 1.0/ray_dir);
}

struct StackNode {
	vec3 origin;
	int index;
	int hit;
	int subvoxel_index;
};

struct Node {
	int material_id;
	int sub_voxels[8];
};

#define MAX_DEPTH 12
#define EPS 1e-8
const float infinity = 1. / 0.;


const int SOLID = 1;
const int EMPTY = 0;

Node[] data = Node[](
	Node(0, int[](1, 2, 3, 4, 5, 6, 7, 8)),
	Node(0, int[](9, 0, 0, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 9, 0, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 9, 0, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 9, 0, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 9, 0, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 9, 0, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 0, 9, 0)),
	Node(0, int[](0, 0, 0, 0, 0, 0, 0, 9)),
	Node(1, int[](0, 0, 0, 0, 0, 0, 0, 0))
	//Node(1, int[](0, 0, 0, 0, 0, 0, 0, 0))
);

float max_component(vec3 v) {
	return max(v.x, max(v.y, v.z));
}

float min_component(vec3 v) {
	return min(v.x, min(v.y, v.z));
}

StackNode stack[12];
int currentStackIndex = 0;
StackNode currentStack = StackNode(vec3(0), 0, 0, 0);
float size = 1.0;

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

int getSubvoxel(float tsMin, vec3 tZero, int tsMinFace, vec3 dir) {
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
	int tsMinFace = getFace(tMinMask, tMin, tsMin);
	
	float tsMax = min_component(tMax);
	int tsMaxFace = getFace(tMaxMask, tMax, tsMax);
	
	if (tsMin <= tsMax) {
		vec3 hitpoint = point + r.dir*tsMin + currentStack.origin;
		vec3 tZero = (-point)*r.invDir;
		int subvoxel = getSubvoxel(tsMin, tZero, tsMinFace, r.dir);
		
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



void main()
{
	Ray ray = generate_ray();
	
	bool hit = false;
	for(int i = 0; i < 1000; i++) {
		if((currentStack.hit & 1) == 0) {
			currentStack.hit = intersect(ray);
		}
		if((currentStack.hit & 2) != 0) {
			if(data[currentStack.index].material_id == SOLID) {
				hit = true;
				break;
			} else {
				int subvoxel = getNthSubvoxel(currentStack.hit, currentStack.subvoxel_index);
				if(subvoxel != (-1)) {
					if(data[currentStack.index].sub_voxels[subvoxel] != 0) {
						stack[currentStackIndex] = currentStack;
						currentStackIndex++;
						size /= 2;
						currentStack.origin += (2 * vec3((subvoxel & 1), (subvoxel & 2) >> 1, (subvoxel & 4) >> 2) - 1) * size;
						currentStack.index = data[currentStack.index].sub_voxels[subvoxel];
						currentStack.subvoxel_index = 0;
						currentStack.hit = 0;
					} else {
						currentStack.subvoxel_index++;
					}
					
				} else if(currentStackIndex != 0) {
					currentStackIndex -= 1;
					currentStack = stack[currentStackIndex];
					currentStack.subvoxel_index++;
					size *= 2;
				} else {
					break;
				}
			}
		} else {
			break;
		}
	}
    outColor = vec4(vec3(hit), 0.5);
}