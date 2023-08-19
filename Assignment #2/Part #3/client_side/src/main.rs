extern crate server_side;
use server_side::*;

// can comment out accordingly to get desired result

fn main() {
    // ANNOTATE THE VIDEO
    annotate(true);

    // CHANGE THE KEY TO END STREAMING
    terminate(97);

    // SETTING DELAY BETWEEN READ/WRITE
    //      this works best in conjunction with the ping call
    //      and adjusting the delay to be similar to that of the
    //      connection that is established between the client/server side
    //      and the remote server
    delay(0);

    // DISPLAY THE FEED
    display();
}
