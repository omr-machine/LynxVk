
// Input structure
struct PSInput {
    [[vk::binding(0, 0)]]
    float3 color;
};

// Output structure
struct PSOutput {
    [[vk::binding(0, 0)]]
    float4 color : SV_Target0;
};

// Pixel shader using structures
[shader("pixel")]
PSOutput main(PSInput input) {
    PSOutput output;
    output.color = float4(input.color, 1.0);
    // output.color = float4(1.0, 1.0, 0.0, 1.0);
    
    return output;
}


// [shader("pixel")]
// float4 main(float3 inColor) : SV_Target0 {
//     return float4(inColor, 1.0);
// }
