[vk::push_constant] float tesselationValue;

struct PerQuad {
    float4 position : SV_Position;
};

struct PatchTessFactors {
    float[4] tessEdge   : SV_TessFactor;
    float[2] tessInside : SV_InsideTessFactor;
};

struct InvocationID {
    uint id : SV_OutputControlPointID; //SV_GSInstanceID
};

PatchTessFactors PatchConstantFunction(InputPatch<PerQuad, 16> patch) {
    PatchTessFactors tessFactors;

    [unroll]
    for (uint i = 0; i < 2; i++) {
        tessFactors.tessEdge[i]       = tesselationValue;
        tessFactors.tessEdge[i + 2]   = tesselationValue;
        tessFactors.tessInside[i]     = tesselationValue;
    }

    return tessFactors;
}

[shader("hull")]
[domain("quad")]
[outputcontrolpoints(16)]
[patchconstantfunc("PatchConstantFunction")]
PerQuad main(InputPatch<PerQuad, 16> patch, InvocationID invocationID) {
    PerQuad output;
    output.position = patch[invocationID.id].position;
    return output;
}
