#!/usr/bin/env python3

import os
import cv2
import torch
import torch.nn as nn
import torch.nn.functional as F
import sys
import torch.optim as optim
import numpy as np
import random
import matplotlib.pyplot as plt

N = 1
class Net(nn.Module):

    def __init__(self):
        super(Net, self).__init__()
        self.fc1 = nn.Linear(N**2+1, 1)  # 5*5 from image dimension
        # self.fc2 = nn.Linear(2, 1)

    def forward(self, x):
        # Max pooling over a (2, 2) window
        # x = torch.flatten(x, 1) # flatten all dimensions except the batch dimension
        x = F.sigmoid(self.fc1(x))
        # x = F.sigmoid(self.fc2(x))
        return x


def augment(image):
    new_images = []
    for i in range(4):
        new_images.append(image)
        new_images.append(np.flip(image))
        image = np.rot90(image)
    return new_images

        

net = Net()
print(net)


image_and_colors = []
labels = []
for path in os.listdir('black_pieces'):
    img = cv2.imread('black_pieces/' + path, 0)
    image_and_colors.append((img, 'white' in path))
    # img = cv2.resize(img, (N, N)).flatten()
    # feature[:N**2] = img/255.0
    # features.append(feature)
    labels.append(0)

for path in os.listdir('white_pieces'):
    img = cv2.imread('white_pieces/' + path, 0)
    image_and_colors.append((img, int('white' in path)))
    # img = cv2.resize(img, (N, N)).flatten()
    # feature[:N**2] = img/255.0
    labels.append(1)

train_data = []
for ((img, color), label) in zip(image_and_colors, labels):
    img = cv2.resize(img, (N, N))
    for image in augment(img):
        f = np.zeros(N**2+1)
        f[-1] = float(color)
        f[:N**2] = image.flatten()/255.0
        train_data.append((f, label))

# train_data = list(zip(features, labels))
random.shuffle(train_data)
validation = train_data[-80:]
train_data = train_data[:-80]
# features = 
criterion = nn.MSELoss()
optimizer = optim.Adam(net.parameters(), lr=0.08)

X = torch.tensor([f for (f, _) in train_data]).float()
Y = torch.tensor([[label] for (_, label) in train_data]).float()

# in your training loop:
errors = []
val_errors = []
for i in range(1000):
    mse = 0
    # for (feature, label) in train_data:
    
    optimizer.zero_grad()   # zero the gradient buffers
    output = net(X)
    # print(output.shape, Y.shape)
    loss = criterion(output, Y)
    loss.backward()
    optimizer.step()    # Does the update
    mse += loss.item()
    errors.append(mse)

    valerr = 0
    for (feature, label) in validation:
        output = net(torch.from_numpy(feature).float())
        loss = criterion(output, torch.tensor([label]).float())
        valerr += loss.item()

    val_errors.append(valerr/len(validation))

print(net.fc1.weight)
print(net.fc1.bias)
# print(net.fc2.weight)
# print(net.fc2.bias)

plt.grid(True)
plt.yscale('log')
plt.plot(val_errors)
plt.plot(errors)
plt.legend(["validatin", "errors"])
plt.show()