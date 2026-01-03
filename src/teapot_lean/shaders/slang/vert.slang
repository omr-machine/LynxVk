
struct ControlPoint
{
    float x;
    float y;
    float z;
};

[[vk::binding(0, 0)]]
StructuredBuffer<ControlPoint> controlPointBuffer;

[shader("vertex")]
float4 main(uint vertexID : SV_VertexID) : SV_Position {
    ControlPoint cp = controlPointBuffer[vertexID];
    // cp.x += 3.0;
    // cp.z *= -1.0;

    return float4(cp.x, cp.y, cp.z, 1.0);
}


