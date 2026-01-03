struct PerQuad {
    float4 position : SV_Position;
};

struct TESIn {
    float2 tessCoord : SV_DomainLocation;
    uint primitiveID : SV_PrimitiveID;
};
struct TESEOut {
    float4 position : SV_Position;
    float3 color;
};

struct PatchData
{
    float4x4 transform;
    float4 color;
};

[[vk::binding(1, 0)]] // layout(set = 0, binding = 1)
StructuredBuffer<PatchData> patchData;

[[vk::binding(2, 0)]]
ConstantBuffer<float4x4> mvpMatrix;

[shader("domain")]
[domain("quad")]
TESEOut main(const OutputPatch<PerQuad, 16> patch, TESIn input) {
    float4 basisU = bernsteinBasis(input.tessCoord.x);
    float4 basisV = bernsteinBasis(input.tessCoord.y);

    float4 localPos = evaluateBezier(basisU, basisV, patch);
    localPos = mul(localPos, mul(patchData[input.primitiveID].transform, mvpMatrix));

    TESEOut out;
    out.position = localPos;
    out.color = patchData[input.primitiveID].color.xyz;
    return out;
}


float4 bernsteinBasis(float t)
{
    float invT = 1.0f - t;

    return float4(
        invT * invT * invT,     // (1-t)^3
        3.0f * t * invT * invT, // 3t(1-t)^2
        3.0f * t * t * invT,    // 3t2(1-t)
        t * t * t               // t3
    );
}

float4 evaluateBezier(float4 basisU, float4 basisV, const OutputPatch<PerQuad, 16> patch)
{
    float4 value = float4(0.0, 0.0, 0.0, 0.0);

    value = basisV.x * (
        patch[0].position * basisU.x + patch[1].position * basisU.y + 
        patch[2].position * basisU.z + patch[3].position * basisU.w
    );
    value += basisV.y * (
        patch[4].position * basisU.x + patch[5].position * basisU.y + 
        patch[6].position * basisU.z + patch[7].position * basisU.w
    );
    value += basisV.z * (
        patch[8].position * basisU.x + patch[9].position * basisU.y + 
        patch[10].position * basisU.z + patch[11].position * basisU.w
    );
    value += basisV.w * (
        patch[12].position * basisU.x + patch[13].position * basisU.y + 
        patch[14].position * basisU.z + patch[15].position * basisU.w
    );
    value.w = 1.0;

    return value;
}

