// Aggregates the LiteRT 2.x C API surface for bindgen.
//
// Pinned to LiteRT v2.1.4 headers vendored at
//   third_party/litert-v2.1.4/litert/c/
//
// GL / OpenCL / WebGPU support is intentionally left OFF
// (LITERT_HAS_{OPENGL,OPENCL,WEBGPU}_SUPPORT undefined) so bindgen parses
// without platform-specific dependencies; the matching runtime symbols are
// still exported by libLiteRt and reachable via the opaque-typedef bindings.

#ifndef LITERT_SYS_WRAPPER_H_
#define LITERT_SYS_WRAPPER_H_

// Core
#include "litert/c/litert_common.h"
#include "litert/c/litert_any.h"
#include "litert/c/litert_layout.h"
#include "litert/c/litert_platform_support.h"

// Environment / options
#include "litert/c/litert_environment_options.h"
#include "litert/c/litert_environment.h"
#include "litert/c/litert_opaque_options.h"
#include "litert/c/litert_options.h"

// Model / ops
#include "litert/c/litert_model_types.h"
#include "litert/c/litert_model.h"
#include "litert/c/litert_op_code.h"
#include "litert/c/litert_op_options.h"

// Tensor buffers (and the opaque GL/OpenCL/WebGPU typedef shims they pull in)
#include "litert/c/litert_gl_types.h"
#include "litert/c/litert_opencl_types.h"
#include "litert/c/litert_webgpu_types.h"
#include "litert/c/litert_tensor_buffer_types.h"
#include "litert/c/litert_custom_tensor_buffer.h"
#include "litert/c/litert_tensor_buffer_requirements.h"
#include "litert/c/litert_tensor_buffer.h"

// Compiled model + async execution
#include "litert/c/litert_event_type.h"
#include "litert/c/litert_event.h"
#include "litert/c/litert_compiled_model.h"

// Per-backend compilation options (public subset)
#include "litert/c/options/litert_compiler_options.h"
#include "litert/c/options/litert_cpu_options.h"
#include "litert/c/options/litert_gpu_options.h"
#include "litert/c/options/litert_runtime_options.h"

// Profiling / metrics
#include "litert/c/litert_profiler_event.h"
#include "litert/c/litert_metrics.h"

// Logging control. Lives under `internal/` in the upstream header layout but
// every function below is exported from libLiteRt.* and intended for
// programmatic use (severity filtering, custom loggers).
#include "litert/c/internal/litert_logging.h"

#endif  // LITERT_SYS_WRAPPER_H_
