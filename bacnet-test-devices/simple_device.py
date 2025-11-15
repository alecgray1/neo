#!/usr/bin/env python3
"""
Simple BACnet device for Docker deployment
Accepts device configuration via environment variables
"""
import asyncio
import sys
import os
from bacpypes3.argparse import SimpleArgumentParser
from bacpypes3.app import Application
from bacpypes3.local.device import DeviceObject
from bacpypes3.local.analog import AnalogInputObject, AnalogOutputObject, AnalogValueObject
from bacpypes3.local.binary import BinaryInputObject
from bacpypes3.primitivedata import Real
from bacpypes3.basetypes import EngineeringUnits


async def main():
    # Get configuration from environment variables or use defaults
    device_name = os.getenv("DEVICE_NAME", "Device-1")
    device_instance = int(os.getenv("DEVICE_INSTANCE", "101"))
    device_type = os.getenv("DEVICE_TYPE", "VAV")
    ip_address = os.getenv("IP_ADDRESS", "0.0.0.0")

    print(f"Starting BACnet {device_type}: {device_name} (instance {device_instance}) on {ip_address}:47808")

    # Use BACpypes3 argument parser to properly initialize
    parser = SimpleArgumentParser()
    args = parser.parse_args([
        "--name", device_name,
        "--instance", str(device_instance),
        "--address", f"{ip_address}/24:47808"
    ])

    # Create application using from_args for proper initialization
    app = Application.from_args(args)

    if device_type == "VAV":
        # VAV devices: AI:1 (temp), AO:1 (damper), BI:1 (occupancy), AV:1 (setpoint)
        temp_sensor = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-Temperature",
            presentValue=Real(73.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Zone temperature sensor",
        )
        app.add_object(temp_sensor)

        damper = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-Damper",
            presentValue=Real(45.0),
            units=EngineeringUnits.percent,
            description="Damper position control",
        )
        app.add_object(damper)

        occupancy = BinaryInputObject(
            objectIdentifier=("binaryInput", 1),
            objectName=f"{device_name}-Occupancy",
            presentValue="active",
            description="Occupancy sensor",
        )
        app.add_object(occupancy)

        setpoint = AnalogValueObject(
            objectIdentifier=("analogValue", 1),
            objectName=f"{device_name}-Setpoint",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Temperature setpoint",
        )
        app.add_object(setpoint)

        print(f"✓ Created VAV with 4 objects: AI:1, AO:1, BI:1, AV:1")

    elif device_type == "AHU":
        # AHU devices: AI:1 (supply temp), AI:2 (return temp), BV:1 (fan status), AO:1 (fan speed)
        supply_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 1),
            objectName=f"{device_name}-SupplyTemp",
            presentValue=Real(55.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Supply air temperature",
        )
        app.add_object(supply_temp)

        return_temp = AnalogInputObject(
            objectIdentifier=("analogInput", 2),
            objectName=f"{device_name}-ReturnTemp",
            presentValue=Real(72.0),
            units=EngineeringUnits.degreesFahrenheit,
            description="Return air temperature",
        )
        app.add_object(return_temp)

        fan_speed = AnalogOutputObject(
            objectIdentifier=("analogOutput", 1),
            objectName=f"{device_name}-FanSpeed",
            presentValue=Real(75.0),
            units=EngineeringUnits.percent,
            description="Fan speed control",
        )
        app.add_object(fan_speed)

        print(f"✓ Created AHU with 3 objects: AI:1, AI:2, AO:1")

    print(f"✓ {device_name} is ready and listening for BACnet requests")
    print(f"  Device Instance: {device_instance}")
    print(f"  Listening on: {ip_address}:47808")
    sys.stdout.flush()

    # Run forever
    await asyncio.Future()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print(f"\nShutting down...")
        sys.exit(0)
