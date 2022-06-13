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
	int advance_inside;
	int index;
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
);

float max_component(vec3 v) {
	return max(v.x, max(v.y, v.z));
}

float min_component(vec3 v) {
	return min(v.x, min(v.y, v.z));
}

float mid_component(vec3 v) {
	float a = v.x;
	float b = v.y;
	float c = v.z;
	if (a > b)
    {
        if (b > c)
            return b;
        else if (a > c)
            return c;
        else
            return a;
    }
    else
    {
        // Decided a is not greater than b.
        if (a > c)
            return a;
        else if (b > c)
            return c;
        else
            return b;
    }
}


int get_subvoxel(vec3 point, float size, bool check_bounds) {
	float dist = max_component(abs(point));
	if(check_bounds && (dist > size)) {
		return -1;
	} else {
		return (point.x > 0 ? 1 : 0) | (point.y > 0 ? 2 : 0) | (point.z > 0 ? 4 : 0);
	}
}

int not_inside(vec3 point, float size) {
	return max_component(abs(point)) > size ? 1 : 0;
}

StackNode stack[12];
int currentStackIndex = 0;
StackNode currentStack = StackNode(vec3(0), 0, 0, -1);
float size = 1.0;

float intersect(Ray r) {	
	vec3 point = r.origin-currentStack.origin;
	if(currentStack.advance_inside != 0) {
		vec3 t0 = (-point) * r.invDir;
		vec3 t1 = t0;
		t0 += big * vec3(not_inside(point+t1.x*r.dir, size), not_inside(point+t1.y*r.dir, size), not_inside(point+t1.z*r.dir, size));
		float t;
		if(currentStack.advance_inside == 1) {
			t = min_component(t0);
			if(t == t1.x) {
				currentStack.subvoxel_index = 1 ^ currentStack.subvoxel_index;
			} else if(t == t1.y) {
				currentStack.subvoxel_index = 2 ^currentStack.subvoxel_index;
				
			} else if(t == t1.z) {
				currentStack.subvoxel_index = 4 ^ currentStack.subvoxel_index;
			} else {
				currentStack.subvoxel_index = -1;
				return -1;
			}
			//currentStack.advance_inside += 1;
			if(data[currentStack.index].sub_voxels[currentStack.subvoxel_index] == 0) {
				currentStack.advance_inside += 1;
			}
		} 
		if(currentStack.advance_inside == 2) {
			t = mid_component(t0);
			if(t == t1.x) {
				currentStack.subvoxel_index = 1 ^ currentStack.subvoxel_index;
			} else if(t == t1.y) {
				currentStack.subvoxel_index = 2 ^currentStack.subvoxel_index;
			} else if(t == t1.z) {
				currentStack.subvoxel_index = 4 ^ currentStack.subvoxel_index;
			} else {
				currentStack.subvoxel_index = -1;
				return -1;
			}
			//currentStack.advance_inside += 1;
			if(data[currentStack.index].sub_voxels[currentStack.subvoxel_index] == 0) {
				currentStack.advance_inside += 1;
			}
		}  
		if(currentStack.advance_inside == 3) {
			t = max_component(t0);
			if(t == t1.x) {
				currentStack.subvoxel_index = 1 ^ currentStack.subvoxel_index;
			} else if(t == t1.y) {
				currentStack.subvoxel_index = 2 ^currentStack.subvoxel_index;
			} else if(t == t1.z) {
				currentStack.subvoxel_index = 4 ^ currentStack.subvoxel_index;
			} else {
				currentStack.subvoxel_index = -1;
				return -1;
			}
			//currentStack.advance_inside += 1;
			if(data[currentStack.index].sub_voxels[currentStack.subvoxel_index] == 0) {
				currentStack.advance_inside += 1;
			}
		} else if(currentStack.advance_inside >= 4) {
			currentStack.subvoxel_index = -1;
			return -1;
		}
		currentStack.advance_inside += 1;
		return t;
	}
	
	vec3 t0 = (-size-point) * r.invDir;
	vec3 t2 = (size-point) * r.invDir;
	float tmin = max_component(min(t0, t2)), tmax = min_component(max(t0, t2));
	if(tmin <= tmax) {
		currentStack.subvoxel_index = get_subvoxel(point + tmin*r.dir, size, false);
		currentStack.advance_inside += 1;
	} else {
		currentStack.subvoxel_index = -1;
		tmin = -1;
	}
	return tmin;
}



void main()
{
	Ray ray = generate_ray();
	
	bool hit = false;
	for(int i = 0; i < 1000; i++) {
		float dist = intersect(ray);
		if(dist != -1 && data[currentStack.index].material_id == SOLID) {
			hit = true;
			break;
		} else if(currentStack.subvoxel_index != -1) {
			if(data[currentStack.index].sub_voxels[currentStack.subvoxel_index] != 0) {
				stack[currentStackIndex] = currentStack;
				currentStackIndex++;
				size /= 2;
				currentStack.origin += (2 * vec3((currentStack.subvoxel_index & 1), (currentStack.subvoxel_index & 2) >> 1, (currentStack.subvoxel_index & 4) >> 2) - 1) * size;
				currentStack.index = data[currentStack.index].sub_voxels[currentStack.subvoxel_index];
				currentStack.advance_inside = 0;
				currentStack.subvoxel_index = -1;
			}

		} else if(currentStackIndex != 0) {
			currentStackIndex -= 1;
			currentStack = stack[currentStackIndex];
			size *= 2;
		} else {
			break;
		}
	}
	
    outColor = vec4(vec3(hit), 0.5);
}