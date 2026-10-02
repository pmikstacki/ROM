# ROM demo

Reserved for a small application built on ROM's public interfaces.

Implementation starts only after the first core milestone is ready: resource and field contracts, action validation, durable persistence, committed events, and recoverable reactions pass their acceptance scenarios.

The demo will test how easily an application can be built without bypassing the core. Its domain and transport will be chosen later; no application framework, server, or domain implementation is introduced now.

Success means a reader can define a resource, mutate it through an action, observe a committed event, and implement a reaction using only documented public interfaces. Demo feedback should improve ROM's interfaces rather than produce demo-specific shortcuts in the core.
