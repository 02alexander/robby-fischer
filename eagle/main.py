#!/usr/bin/env python3

import numpy as np
import cv2
import shapely

import freenect as fn


# Camera properties
CAMERA_MATRIX = np.array([[540.39224345, 0, 289.60021054], [0, 541.2481217, 249.74312654], [0, 0, 1]])
CAMERA_DISTORTION = np.array([[-0.06999264,  0.05139885, -0.00407623, -0.00333564,  0.10434826]])

# The markers on the board.
MARKERS = {
    6: [-0.4, -0.4, 0],
    9: [ 8.4,  8.4, 0],
    12: [ 8.4, -0.4, 0],
    15: [-0.4,  8.4, 0],
}
MARKER_IDS = sorted(MARKERS)
MARKER_POINTS = np.array([MARKERS[id] for id in MARKER_IDS])

def find_transform(img):
    """Finds the transform for the chessboard. At least four markers need to be visible."""

    aruco_dict = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_4X4_50)
    parameters =  cv2.aruco.DetectorParameters()
    corners, ids, rejectedImgPoints = cv2.aruco.detectMarkers(img, aruco_dict, parameters=parameters)

    if ids is None:
        return None, None

    midpoints = {}
    for i in range(len(ids)):
        corner = corners[i]
        midpoint = np.mean(corner[0], axis=0)
        midpoints[ids[i][0]] = midpoint

    points = [midpoints[id] for id in MARKER_IDS if id in midpoints]
    if len(points) < 4:
        return None, None

    ret, rvec, tvec = cv2.solvePnP(MARKER_POINTS, np.array(points), CAMERA_MATRIX, CAMERA_DISTORTION)

    if not ret:
        return None, None

    return rvec, tvec

def corner_position(rank, file, height, rvec, tvec):
    """Finds the position of the specified intersection in the coordinate system of the camera."""
    world_point = np.array([[[file,rank,height]]], dtype='float32')
    return cv2.projectPoints(
        world_point, rvec, tvec, CAMERA_MATRIX, CAMERA_DISTORTION
    )[0][0][0]

def main():
    """Main function"""
    # cap = cv2.VideoCapture(2)
    stop = False
    while True:
        k = cv2.waitKey(1)
        if k == 27:
            break
        if k == ord('s'):
            stop = True
        if k == ord('r'):
            stop = False
        if stop:
            continue

        print("get img")
        res = fn.sync_get_video()
        if res:
            (img, vtimestamp) = res
            cv2.imshow("window", img)
            print("got img")


        # ret, img = cap.read()
        # if not ret:
            # break

        # gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
        # edges = cv2.Canny(gray, 100, 250)



        # rvec, tvec = find_transform(img)
        # if rvec is not None and tvec is not None:
        #     for rank in range(9):
        #         for file in range(9):
        #             point = corner_position(rank, file, 0, rvec, tvec)
        #             cv2.circle(edges, (int(point[0]), int(point[1])), 8, color=[255, 255, 255])
            
        #     for rank in range(8):
        #         for file in range(8):
        #             bottom = shapely.geometry.Polygon([
        #                 corner_position(rank + 0.1, file + 0.1, 0, rvec, tvec),
        #                 corner_position(rank + 0.1, file + 0.9, 0, rvec, tvec),
        #                 corner_position(rank + 0.9, file + 0.9, 0, rvec, tvec),
        #                 corner_position(rank + 0.9, file + 0.1, 0, rvec, tvec),
        #             ])
        #             top = shapely.geometry.Polygon([
        #                 corner_position(rank - 0.2, file - 0.2, 1.4, rvec, tvec),
        #                 corner_position(rank - 0.2, file + 1.2, 1.4, rvec, tvec),
        #                 corner_position(rank + 1.2, file + 1.2, 1.4, rvec, tvec),
        #                 corner_position(rank + 1.2, file - 0.2, 1.4, rvec, tvec),
        #             ])

        #             intersection = list(bottom.intersection(top).exterior.coords)
                    
                    

        #             for p1, p2 in zip(intersection, intersection[1:]):
        #                 cv2.line(edges, (int(p1[0]), int(p1[1])), (int(p2[0]), int(p2[1])), color=255)

        #             #print(list(intersection.coords))
                    
        #             # draw polygons from bottom and top
        #             # for i in range(4):
        #             #     cv2.line(img, (int(bottom[i][0]), int(bottom[i][1])), (int(bottom[(i+1)%4][0]), int(bottom[(i+1)%4][1])), color=[255, 0, 0])
        #             #     cv2.line(img, (int(top[i][0]), int(top[i][1])), (int(top[(i+1)%4][0]), int(top[(i+1)%4][1])), color=[0, 255, 0])
        #                 # cv2.line(edges, (int(bottom[i][0]), int(bottom[i][1])), (int(top[i][0]), int(top[i][1])), color=[255, 255, 255])

            



        # blurred = cv2.blur(img, (30, 30))
        # # red = np.array(blurred[:,:,2]/np.sum(blurred, axis=2) > 0.66, dtype='float32')

        # normalized = img/np.sum(blurred, axis=2)[:,:,np.newaxis]
    


        # ## use the hough transform to find the ellipses
        # # ellipses = cv2.HoughCircles(gray, cv2.HOUGH_GRADIENT, 1, 20, param1=40, param2=25, minRadius=5, maxRadius=20)
        # # if ellipses is not None:
        # #     ellipses = np.uint16(np.around(ellipses))
        # #     for ellipse in ellipses[0,:]:
        # #         cv2.circle(img, (ellipse[0], ellipse[1]), ellipse[2], (0,255,0), 2)
        # #         cv2.circle(img, (ellipse[0], ellipse[1]), 2, (0,0,255), 3)
            
        # thresh = cv2.adaptiveThreshold(gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, 1, 11, 0)

        # cv2.imshow("window", edges)



if __name__ == "__main__":
    main()