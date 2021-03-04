package vqueue.example;

import org.sm.vqueue.VQueue;

public class App {
    public static void main(String[] args) {
        VQueue h = new VQueue();
        h.setNameAndPath("example_queue", "./data/out");
        System.out.println("write test queue...");
        for (int i = 0; i < 1000000; i++) {
            h.push("tst_" + i);
        }
    }
}
