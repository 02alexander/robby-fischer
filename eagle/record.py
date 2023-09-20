#!/usr/bin/env python3

import cv2
import freenect as fn

for i in range(10**100):    
    (img, vtimestamp) = fn.sync_get_video()
    cv2.imshow("window", img)
    cv2.waitKey(1000)

    cv2.imwrite(f"images/{i}.png", img)
    
