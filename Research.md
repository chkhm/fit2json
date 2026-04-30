# Research on Garmin FIT files

## Overview of FIT file format

A Garmin FIT (Flexible and Interoperable Data Transfer) file is a binary format that stores fitness and activity data (like GPS coordinates, heart rate, and power). It consists of a standard file header, followed by a sequence of Definition Messages and Data Messages, and ends with a Cyclic Redundancy Check (CRC). [1, 2, 3, 4] 
Here is a detailed breakdown of its structure:

## 1. File Header [4] 

Every FIT file begins with a fixed-length header (usually 14 bytes) that contains essential metadata: [4, 5, 6] 

* Header Size: Length of the header (e.g., 12 or 14 bytes).
* Protocol Version: The FIT profile version being used.
* Profile Version: Specific version of the FIT profile.
* File Size: The size of the data records in bytes.
* Data Type: String indicating the file type (.FIT). [7, 8, 9, 10, 11] 

## 2. Message Architecture

The core of the file is a continuous stream of messages representing the activity over time. These come in two forms: [12] 

* Definition Messages: Describe the structure of the data that is about to follow (e.g., specifying that a "Record" message will contain GPS coordinates, Heart Rate, and Altitude).
* Data Messages: The actual, populated data that adheres to the preceding definition. [1, 2, 13, 14, 15] 

## 3. Key Message Types

Within an Activity FIT file, data is written chronologically using specific message types: [2, 13, 16] 

* File ID: The very first message. Identifies the file type (Activity, Course, Workout), manufacturer, device serial number, and product number.
* Device Info: Contains details about the hardware recording the data, its version, and battery status.
* Session: A summary message that encapsulates an entire workout, including total time, distance, calories, and average heart rate.
* Lap: Summarizes specific segments (e.g., 1-mile splits or manually triggered laps).
* Record: The timestamped, real-time metrics recorded every second (GPS, heart rate, cadence, power, etc.).
* Event: Records specific occurrences like pausing the timer, starting a lap, or powering off.
* Developer Data Id: Contains identifying information if custom Connect IQ data fields are being tracked. [2, 16, 17, 18, 19] 

## 4. File Footer (CRC)

The file ends with a 2-byte CRC (Cyclic Redundancy Check). This is used to verify the integrity of the file, ensuring that the data was not corrupted during saving or transferring. [4, 20] 

------------------------------

Note: Because FIT is a highly compressed binary file, you cannot read it with a basic text editor. To view or manipulate its contents, developers use Garmin's official FIT SDK (available in languages like C++, C#, Java, and Python) or third-party decoding tools. [10, 11, 12, 16, 21] 

[1] [https://developer.garmin.com](https://developer.garmin.com/fit/protocol/#:~:text=A%20FIT%20file%20contains%20a%20series%20of,a%20series%20of%20data%2Dfilled%20fields%20%28Figure%202%29.)
[2] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/decoding-activity-files/#:~:text=FIT%20Activity%20files%20are%20used%20to%20store,Lap%20*%20Length%20*%20Event%20*%20Record)
[3] [https://medium.com](https://medium.com/decathlondigital/gpx-tcx-fit-how-to-choose-the-best-file-extension-for-sport-activity-transfer-403487337c04)
[4] [https://developer.garmin.com](https://developer.garmin.com/fit/file-types/#:~:text=FIT%20File%20Structure%20All%20FIT%20files%20have,Message%20Definitions%2C%20Messages%2C%20and%20a%202%2Dbyte%20CRC.)
[5] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/isfit-checkintegrity-read/)
[6] [https://github.com](https://github.com/inonoob/Coxswain2Fit#:~:text=Fit%20file%20structure%20It%20has%20a%20Header,is%20used%20to%20check%20the%20file%20integrity.)
[7] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/isfit-checkintegrity-read/)
[8] [https://github.com](https://github.com/inonoob/Coxswain2Fit#:~:text=Fit%20file%20structure%20It%20has%20a%20Header,is%20used%20to%20check%20the%20file%20integrity.)
[9] [https://blog.studioblueplanet.net](https://blog.studioblueplanet.net/software/java-garminant-fit-file-reader)
[10] [https://www.thisisant.com](https://www.thisisant.com/forum/viewthread/4275)
[11] [https://developer.garmin.com](https://developer.garmin.com/fit/)
[12] [https://github.com](https://github.com/muktihari/fit)
[13] [https://www.fitfileviewer.com](https://www.fitfileviewer.com/#:~:text=A%20FIT%20file%20is%20a%20structured%20file,as%20time%2C%20distance%2C%20heart%20rate%2C%20and%20position.)
[14] [https://hexdocs.pm](https://hexdocs.pm/fit_decoder/readme.html#:~:text=The%20primary%20goal%20of%20this%20library%20is,basic%20activity%20tracking%20to%20advanced%20physiological%20metrics.)
[15] [https://peateasea.de](https://peateasea.de/analysing-fit-data-with-perl-basic-beginnings/#:~:text=A%20FIT%20file%20has%20a%20well%2Ddefined%20structure,data%20fields%20storing%20a%20ride%27s%20various%20parameters.)
[16] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/encoding-activity-files/)
[17] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/developer-data/)
[18] [https://developer.garmin.com](https://developer.garmin.com/fit/file-types/workout/#:~:text=Workout%20File%20Structure%20A%20Workout%20file%20must,then%20one%20or%20more%20Workout%20Step%20messages.)
[19] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/encoding-course-files/)
[20] [https://github.com](https://github.com/inonoob/Coxswain2Fit#:~:text=It%20has%20a%20Header%20with%20a%20size,is%20used%20to%20check%20the%20file%20integrity.)
[21] [https://rpubs.com](https://rpubs.com/kizen777/1124320)

------------------------------
------------------------------

## What is the hierarchy of a fit file?

While a Garmin FIT file is recorded as a continuous, chronological stream of flat binary messages, it maintains a strict logical hierarchy. This hierarchy allows platforms like Strava or Garmin Connect to group micro-level tracking data into macro-level session summaries. [1, 2, 3, 4] 
The conceptual structure of a typical multi-sport (or single-sport) activity file is organized according to this specific data hierarchy:

## 1. File Level (The Root)

This is the master envelope for all data. It is the absolute base of the file and does not repeat. [5] 

* File ID Message: The master descriptor. It identifies the file type (Activity), the device, and the manufacturer. [1] 

## 2. Activity Level

This encapsulates everything that occurred from the moment you pressed Start on your device to the moment you pressed Stop and saved it. [2] 

* Activity Message: Acts as the highest container for the sports performed.
* Primary Attribute: num_sessions (tells platforms how many sessions are nested inside).

## 3. Session Level

A single activity can contain one or more sessions. In a standard workout, there is usually only 1 session. In a triathlon, there would be 5 distinct sessions (Swim, Transition 1, Bike, Transition 2, Run). [4] 

* Session Message: Summarizes a specific sport or leg of a workout.
* Summary Attributes: Total distance, total calories, average heart rate, and overall time. [4, 6] 

## 4. Lap Level

Every Session is broken down into segments called Laps. These can be 1-mile auto-splits or custom segments triggered manually. [1, 4, 7, 8] 

* Lap Message: Summarizes a block of time within a specific session.
* Attributes: Lap number, lap distance, split time, and average speed for just that segment. [3, 4, 7, 8] 

## 5. Record & Event Level (The Leaves)

This is the granular, chronological data recorded every second (or by smart-recording intervals). Hundreds or thousands of these are nested under a single Lap. [1, 9] 

* Record Message: The actual real-time sensor measurements.
* Attributes: Timestamp, GPS coordinates, heart rate, power, cadence, and speed.
* Event Message: Interspersed between records to signal state changes.
* Attributes: Timer pauses, marker drops, or manual laps. [1, 3, 8, 10] 

------------------------------

## 📐 The Hierarchy Visualized

[File ID]
└── [Activity]
    ├── [Session 0] (e.g., Bike Leg)
    │   ├── [Lap 0] (e.g., Mile 1)
    │   │   ├── Record (Timestamp 1, GPS, HR)
    │   │   ├── Record (Timestamp 2, GPS, HR)
    │   │   └── Event (Timer Paused)
    │   └── [Lap 1] (e.g., Mile 2)
    │       ├── Record (Timestamp n, GPS, HR)
    │       └── Event (Timer Resumed)
    └── [Session 1] (e.g., Run Leg)
        └── [Lap 0] (e.g., Mile 1)
            └── Record (Timestamp n, GPS, HR)

------------------------------
If you tell me the specific language you are coding in (such as Python or C#):

* I can provide a functional code template.
* I can show you how to traverse this exact tree using the Garmin FIT SDK.

[1] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/decoding-activity-files/)
[2] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/encoding-activity-files/)
[3] [https://kinni.co](https://kinni.co/what-are-fit-files/)
[4] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/decoding-activity-files/)
[5] [https://developer.garmin.com](https://developer.garmin.com/fit/file-types/)
[6] [https://forum.intervals.icu](https://forum.intervals.icu/t/question-on-fit-file-parsing-misaligned-splits-summary-first-vs-summary-last-handling/123523)
[7] [https://github.com](https://github.com/imgrant/fit2tcx)
[8] [https://www.dcrainmaker.com](https://www.dcrainmaker.com/2014/03/garmin-fenix2-multisport.html/comment-page-2)
[9] [https://developer.garmin.com](https://developer.garmin.com/fit/cookbook/encoding-activity-files/)
[10] [https://www.fitfileviewer.com](https://www.fitfileviewer.com/#:~:text=A%20FIT%20file%20is%20a%20structured%20file,as%20time%2C%20distance%2C%20heart%20rate%2C%20and%20position.)
