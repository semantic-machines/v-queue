use robusta_jni::bridge;

#[bridge]
mod jni {
    use jni::objects::{GlobalRef, JObject};
    use jni::sys::jlong;
    use robusta_jni::convert::JNIEnvLink;
    use robusta_jni::jni::JNIEnv;
    use std::marker::PhantomData;
    use v_queue::queue::Queue;
    use v_queue::record::{Mode, MsgType};

    #[package(org.sm.vqueue)]
    pub struct VQueue<'e, 'a> {
        env: JNIEnv<'e>,
        marker: PhantomData<&'a ()>,
        raw: JObject<'e>,
    }

    impl<'e, 'a> ::robusta_jni::convert::IntoJavaValue<'e> for VQueue<'e, 'a> {
        type Target = JObject<'e>;

        fn into(self, env: &JNIEnv<'e>) -> Self::Target {
            self.raw
        }
    }

    impl<'e, 'a> ::robusta_jni::convert::IntoJavaValue<'e> for &VQueue<'e, 'a> {
        type Target = JObject<'e>;

        fn into(self, env: &JNIEnv<'e>) -> Self::Target {
            self.raw
        }
    }

    impl<'e, 'a> ::robusta_jni::convert::FromJavaValue<'e> for VQueue<'e, 'a> {
        type Source = JObject<'e>;

        fn from(s: Self::Source, env: &JNIEnv<'e>) -> Self {
            VQueue {
                env: env.clone(),
                marker: PhantomData,
                raw: s,
            }
        }
    }

    impl<'e, 'a> JNIEnvLink<'e> for VQueue<'e, 'a> {
        fn get_env(&self) -> &JNIEnv<'e> {
            &self.env
        }
    }

    impl<'env, 'a> VQueue<'env, 'a> {
        #[call_type(unchecked)]
        pub unsafe extern "jni" fn push(mut self, _env: &JNIEnv, val: String) -> i32 {
            if let Some(queue) = get_vqueue(&self) {
                queue.push(val.as_bytes(), MsgType::Object);
            }
            0
        }

        #[call_type(unchecked)]
        pub unsafe extern "jni" fn setNameAndPath(mut self, _env: &JNIEnv, name: String, path: String) -> i32 {
            new_vqueue(&self, &name, &path);
            0
        }

        pub extern "java" fn javaGetQueuePtr(&self) -> ::robusta_jni::jni::errors::Result<i64> {}

        pub extern "java" fn javaSetQueuePtr(&self, i: i64) -> ::robusta_jni::jni::errors::Result<i64> {}
    }

    unsafe fn new_vqueue<'env, 'a>(vqueue: &VQueue, name: &str, path: &str) {
        let queue = Queue::new(path, name, Mode::ReadWrite).expect("!!!!!!!!! FAIL QUEUE");
        let new_ptr = Box::into_raw(Box::new(queue)) as jlong;
        vqueue.javaSetQueuePtr(new_ptr);
    }

    unsafe fn get_vqueue<'env, 'a>(vqueue: &VQueue) -> Option<&'a mut Queue> {
        if let Ok(ptr) = vqueue.javaGetQueuePtr() {
            if ptr != 0 {
                return Some(&mut *(ptr as *mut Queue));
            }
        }
        None
    }
}
