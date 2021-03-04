package org.sm.vqueue;

import java.util.*;

public class VQueue {
    private long queue_ptr;

    public VQueue () {
    }

    public long javaGetQueuePtr() {
        return queue_ptr;
    }

    public long javaSetQueuePtr(long ptr) {
        queue_ptr = ptr;
        return queue_ptr;
    }

    public native int push(String val);
    public native int setNameAndPath(String name, String path);

    static {
        System.loadLibrary("jvqueue");
    }
}
