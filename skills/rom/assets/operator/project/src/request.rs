use rom::operator::{WorkControlOperation, WorkControlRequest, WorkHandle, WorkVersion};

pub fn request() -> WorkControlRequest {
    WorkControlRequest {
        handle: WorkHandle::from_work_id("skill-loopback-fault"),
        expected: WorkVersion {
            generation: "skill-fixture-generation".into(),
            revision: 4,
        },
        key: "skill-original-retry".into(),
        retry_epoch: 7,
        operation: WorkControlOperation::Retry,
    }
}
