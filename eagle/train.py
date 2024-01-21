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

N = 5
N_POINTS = 10
class Net(nn.Module):

    def __init__(self):
        super(Net, self).__init__()
        self.fc1 = nn.Linear(N_POINTS+1, 1)
        # self.fc2 = nn.Linear(3, 1)

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

def extract_features(img):
    pixels = np.sort(img.flatten())
    indices = np.round(np.linspace(0, len(pixels)-1, N_POINTS)).astype(np.int32)
    return pixels[indices]

net = Net()

image_and_colors = []
labels = []
for filename in os.listdir('black_pieces'):
    # if 'white' in filename:
    #     continue
    img = cv2.imread('black_pieces/' + filename, 0)
    image_and_colors.append((img, int('white' in filename)))
    labels.append(0)

for filename in os.listdir('white_pieces'):
    # if 'white' in filename:
    #     continue
    img = cv2.imread('white_pieces/' + filename, 0)
    image_and_colors.append((img, int('white' in filename)))
    labels.append(1)

train_data = []
for ((img, color), label) in zip(image_and_colors, labels):
    img = cv2.resize(img, (N, N))
    for image in augment(img):
        features = np.zeros(N_POINTS+1)
        features[-1] = float(color)
        features[:N_POINTS] = extract_features(image)/255.0
        train_data.append((features, label))


# train_data = list(zip(features, labels))
random.shuffle(train_data)
validation = train_data[-80:]
train_data = train_data[:-80]
# features = 
criterion = nn.MSELoss()
optimizer = optim.Adam(net.parameters(), lr=0.16, weight_decay=1e-5)

X = torch.tensor(np.array([f for (f, _) in train_data])).float()
Y = torch.tensor([[label] for (_, label) in train_data]).float()

# in your training loop:
errors = []
val_errors = []
for i in range(2000):
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

misses = 0
for features, label in train_data:
    output = round(net(torch.from_numpy(features).float())[0].item())
    if output != label:
        misses += 1
print(misses/len(train_data))
    

print(net.fc1.weight)
print(net.fc1.bias)

plt.grid(True)
plt.yscale('log')
plt.plot(val_errors)
plt.plot(errors)
plt.legend(["validatin", "errors"])
plt.show()
