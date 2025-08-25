pub const GEMM_INT8: &str = r#"
__kernel void gemm_int8_relu_q(
    __global const char* A,   // int8: M x K
    __global const char* B,   // int8: K x N
    __global char*       Y,   // int8: M x N (output)
    const int M, const int N, const int K,
    const int lda, const int ldb, const int ldy,
    const int scale_num, const int scale_den // requant: q = (acc * num) / den
) {
    int row = get_global_id(0);
    int col = get_global_id(1);
    if (row >= M || col >= N) return;

    int acc = 0;
    for (int t = 0; t < K; ++t) {
        int a = (int)A[row*lda + t];
        int b = (int)B[t*ldb + col];
        acc += a * b;
    }
    // Requantize to int8 with ReLU
    long tmp = ((long)acc * (long)scale_num) / (long)scale_den;
    if (tmp < 0) tmp = 0;
    if (tmp > 127) tmp = 127;
    Y[row*ldy + col] = (char)tmp;
}
"#;
