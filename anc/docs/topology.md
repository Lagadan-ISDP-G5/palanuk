# Topology

A single wiring topology for all peripherals. Pin numbers in the Pin column refer
to this indexing here: https://vilros.com/pages/raspberry-pi-5-pinout

## All assignments

| Pin | Function | Assignment |   |
| :----: | :-: | :-------: | :-------: |
| 38 | GPIO20, DI | HC-SR04 Echo | Distance measurement |
| 40 | GPIO21, DQ | HC-SR04 Trig | Distance measurement |
| 10 | GPIO15, PWM Output | L298N EN_B -RMTR | Propulsion (Right Motor) |
| 37 | GPIO26, DQ | L298N IN_3 -RMTR | Propulsion (Right Motor) |
| 35 | GPIO19, DQ | L298N IN_4 -RMTR | Propulsion (Right Motor) |
| 12 | GPIO18, PWM Output | L298N EN_A -LMTR | Propulsion (Left Motor) |
| 16 | GPIO23, DQ | L298N IN_1 -LMTR | Propulsion (Left Motor) |
| 18 | GPIO24, DQ | L298N IN_2 -LMTR | Propulsion (Left Motor) |
| 03 | GPIO02, I2C SDA | INA219 and Controller SDA | Power Monitoring |
| 05 | GPIO03, I2C SCL | INA219 and Controller SCL | Power Monitoring |
| 01 | 3.3Vref | I2C Vcc Bus, TTL shifter ref | Power Monitoring |
| 02 | 5Vref | 5Vref | Auxiliary |
| 39 | GND | Logic and power GND ref | Auxiliary |
| 32 | GPIO12, PWM output | SG-90 position command | Camera Panning Servo |


## Auxiliary

## Pin assignment

| Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 01 | 3.3Vref | TTL shifter ref |
| 02 | 5Vref | 5Vref |
| 39 | GND | Logic and power GND ref |

## Distance measurement  

### Pin assignment

| Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 38 | GPIO20, DI | HC-SR04 Echo |
| 40 | GPIO21, DQ | HC-SR04 Trig |

## Propulsion  

### Pin assignment

| Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 10 | GPIO15, PWM Output | L298N EN_B -RMTR |
| 37 | GPIO26, DQ | L298N IN_3 -RMTR |
| 35 | GPIO19, DQ | L298N IN_4 -RMTR |
| 12 | GPIO18, PWM Output | L298N EN_A -LMTR |
| 16 | GPIO23, DQ | L298N IN_1 -LMTR |
| 18 | GPIO24, DQ | L298N IN_2 -LMTR |

## Power Monitoring  

### Pin assignment

| Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 03 | GPIO02, I2C SDA | INA219 and Controller SDA |
| 05 | GPIO03, I2C SCL | INA219 and Controller SCL |
| 01 | 3.3Vref | I2C Vcc Bus |

## Camera Panning  

### Pin assignment

| Pin | Function | Assignment |
| :----: | :-: | :-------: |
| 32 | GPIO12, PWM output | SG-90 position command |
