#!/usr/bin/env python3

import cv2
import sys
import os

def save_image(src_file, dst_folder):
    i = 0
    while True:
        if "white" in src_file:
            path = f'{dst_folder}{i}_white.png'
        if "black" in src_file:
            path = f'{dst_folder}{i}_black.png'
            
        if os.path.isfile(path):
            i += 1
            continue
        os.rename(src_file, path)
        return


def main():
    folder = 'train_images/'
    bi = 0
    for file in os.listdir(folder):
        src_path = folder+file
        image = cv2.imread(src_path)

        while True:
            cv2.imshow("window", image)
            k = cv2.waitKey(0)
            if k == ord('w'):
                save_image(src_path, "white_pieces/")
                break
            elif k == ord('b'):
                save_image(src_path, "black_pieces/")
                break
            elif k == 27:
                return
    

if __name__ == "__main__":
    main()